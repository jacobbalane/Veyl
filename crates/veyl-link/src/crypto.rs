//! Crypto labels and byte layouts are the protocol, including JSON transcript bytes.
use crate::{invalid, Frame, HANDSHAKE, MAX_FRAME, SEALED};
use aes_gcm::{
    aead::{Aead, Payload},
    Aes256Gcm, KeyInit, Nonce,
};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::io;
use subtle::ConstantTimeEq;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

pub const COUNTER_LIMIT: u64 = 1 << 48;

/// Deliberately has no Debug or serialization implementation.
pub struct Secret(pub(crate) Zeroizing<[u8; 32]>);
impl Secret {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(Zeroizing::new(bytes))
    }
    /// Only for key derivation and the platform's protected pairing store.
    pub fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

pub struct Ephemeral {
    private: StaticSecret,
    pub public: [u8; 32],
    pub nonce: [u8; 32],
}
impl Ephemeral {
    pub fn generate() -> io::Result<Self> {
        let mut seed = Zeroizing::new([0; 32]);
        let mut nonce = [0; 32];
        getrandom::fill(seed.as_mut()).map_err(|_| invalid("OS randomness unavailable."))?;
        getrandom::fill(&mut nonce).map_err(|_| invalid("OS randomness unavailable."))?;
        Ok(Self::from_seed(*seed, nonce))
    }
    /// Deterministic construction is for shared test vectors, never live connections.
    pub fn from_seed(seed: [u8; 32], nonce: [u8; 32]) -> Self {
        let private = StaticSecret::from(seed);
        let public = PublicKey::from(&private).to_bytes();
        Self {
            private,
            public,
            nonce,
        }
    }
    pub fn shared(&self, peer: &[u8; 32]) -> io::Result<Secret> {
        let shared = self.private.diffie_hellman(&PublicKey::from(*peer));
        if bool::from(shared.as_bytes().ct_eq(&[0; 32])) {
            return Err(invalid("Invalid X25519 peer."));
        }
        Ok(Secret::new(shared.to_bytes()))
    }
}

pub fn hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update(part);
    }
    hash.finalize().into()
}
pub fn commit(public: &[u8; 32], nonce: &[u8; 32]) -> [u8; 32] {
    hash(&[b"veyl-commit-v1", public, nonce])
}
pub fn mac(key: &[u8; 32], parts: &[&[u8]]) -> [u8; 32] {
    let mut mac =
        <Hmac<Sha256> as hmac::KeyInit>::new_from_slice(key).expect("HMAC accepts 32 bytes");
    for part in parts {
        mac.update(part);
    }
    mac.finalize().into_bytes().into()
}
pub fn matches(a: &[u8; 32], b: &[u8; 32]) -> bool {
    bool::from(a.ct_eq(b))
}

pub struct PairKeys {
    pub link: Secret,
    pub sas_key: Secret,
    pub transcript: [u8; 32],
}
impl PairKeys {
    pub fn derive(shared: &Secret, bodies: &[&[u8]]) -> Self {
        let transcript = hash(bodies);
        let mut okm = Zeroizing::new([0; 64]);
        Hkdf::<Sha256>::new(Some(&transcript), shared.expose())
            .expand(b"veyl pair v1", okm.as_mut())
            .expect("fixed HKDF length");
        Self {
            link: Secret::new(okm[..32].try_into().unwrap()),
            sas_key: Secret::new(okm[32..].try_into().unwrap()),
            transcript,
        }
    }
    pub fn sas(&self) -> String {
        let code = u32::from_be_bytes(
            mac(self.sas_key.expose(), &[b"veyl sas v1"])[..4]
                .try_into()
                .unwrap(),
        ) % 1_000_000;
        format!("{:03} {:03}", code / 1000, code % 1000)
    }
    pub fn confirmation(&self, pc: bool) -> [u8; 32] {
        mac(
            self.link.expose(),
            &[
                if pc {
                    b"veyl confirm pc"
                } else {
                    b"veyl confirm phone"
                },
                &self.transcript,
            ],
        )
    }
}
pub struct SessionKeys {
    pub pc_to_phone: Secret,
    pub phone_to_pc: Secret,
}
impl SessionKeys {
    pub fn derive(shared: &Secret, link: &Secret, bodies: &[&[u8]]) -> Self {
        let transcript = hash(bodies);
        let mut info = b"veyl session v1".to_vec();
        info.extend_from_slice(&transcript);
        let mut okm = Zeroizing::new([0; 64]);
        Hkdf::<Sha256>::new(Some(link.expose()), shared.expose())
            .expand(&info, okm.as_mut())
            .expect("fixed HKDF length");
        Self {
            pc_to_phone: Secret::new(okm[..32].try_into().unwrap()),
            phone_to_pc: Secret::new(okm[32..].try_into().unwrap()),
        }
    }
    pub fn channel(self, pc: bool) -> SealedChannel {
        if pc {
            SealedChannel::new(self.pc_to_phone, self.phone_to_pc)
        } else {
            SealedChannel::new(self.phone_to_pc, self.pc_to_phone)
        }
    }
}

/// Any malformed frame permanently closes the channel and drops both cipher keys.
pub struct SealedChannel {
    tx: Option<Aes256Gcm>,
    rx: Option<Aes256Gcm>,
    send_counter: u64,
    receive_counter: u64,
    close_frame: Option<Frame>,
}
impl SealedChannel {
    pub fn new(tx: Secret, rx: Secret) -> Self {
        Self {
            tx: Some(Aes256Gcm::new(tx.expose().into())),
            rx: Some(Aes256Gcm::new(rx.expose().into())),
            send_counter: 0,
            receive_counter: 0,
            close_frame: None,
        }
    }
    pub fn close(&mut self) {
        self.tx = None;
        self.rx = None;
    }
    pub fn is_closed(&self) -> bool {
        self.tx.is_none()
    }
    /// An authenticated malformed AUDIO block gets one final bad_message before TCP closes.
    pub fn take_close_frame(&mut self) -> Option<Frame> {
        self.close_frame.take()
    }
    pub fn counters(&self) -> (u64, u64) {
        (self.send_counter, self.receive_counter)
    }
    pub fn seal(&mut self, kind: u8, payload: &[u8]) -> io::Result<Frame> {
        let result = self.seal_inner(kind, payload);
        if result.is_err() {
            self.close();
        }
        result
    }
    fn seal_inner(&mut self, kind: u8, payload: &[u8]) -> io::Result<Frame> {
        if self.send_counter >= COUNTER_LIMIT || payload.len() > MAX_FRAME - 26 {
            return Err(invalid("Sealed frame limit."));
        }
        crate::messages::validate_payload(kind, payload)?;
        let mut plain = Vec::with_capacity(payload.len() + 1);
        plain.push(kind);
        plain.extend_from_slice(payload);
        let counter = self.send_counter;
        let (nonce, aad) = headers(counter, payload.len() + 26);
        let cipher = self.tx.as_ref().ok_or_else(|| invalid("Channel closed."))?;
        let encrypted = cipher
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &plain,
                    aad: &aad,
                },
            )
            .map_err(|_| invalid("Encryption failed."))?;
        let mut body = counter.to_be_bytes().to_vec();
        body.extend_from_slice(&encrypted);
        self.send_counter += 1;
        Ok(Frame { kind: SEALED, body })
    }
    pub fn open(&mut self, frame: &Frame) -> io::Result<(u8, Vec<u8>)> {
        let result = self.open_inner(frame);
        if result.is_err() {
            self.close();
        }
        result
    }
    fn open_inner(&mut self, frame: &Frame) -> io::Result<(u8, Vec<u8>)> {
        if frame.kind == HANDSHAKE
            || frame.kind != SEALED
            || frame.body.len() < 25
            || frame.body.len() >= MAX_FRAME
        {
            return Err(invalid("Expected sealed frame."));
        }
        let counter = u64::from_be_bytes(frame.body[..8].try_into().unwrap());
        if counter != self.receive_counter || counter >= COUNTER_LIMIT {
            return Err(invalid("Unexpected sealed counter."));
        }
        let (nonce, aad) = headers(counter, frame.body.len() + 1);
        let cipher = self.rx.as_ref().ok_or_else(|| invalid("Channel closed."))?;
        let plain = cipher
            .decrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &frame.body[8..],
                    aad: &aad,
                },
            )
            .map_err(|_| invalid("Authentication failed."))?;
        let (&kind, payload) = plain
            .split_first()
            .ok_or_else(|| invalid("Empty sealed message."))?;
        if let Err(error) = crate::messages::validate_payload(kind, payload) {
            if kind == crate::AUDIO {
                let notice = crate::messages::Control::Error {
                    code: "bad_message".into(),
                    message: "Invalid audio block.".into(),
                }
                .encode()?;
                self.close_frame = self.seal_inner(crate::CONTROL, &notice).ok();
            }
            return Err(error);
        }
        self.receive_counter += 1;
        Ok((kind, payload.to_vec()))
    }
}
fn headers(counter: u64, length: usize) -> ([u8; 12], [u8; 13]) {
    let mut nonce = [0; 12];
    nonce[4..].copy_from_slice(&counter.to_be_bytes());
    let mut aad = [0; 13];
    aad[..4].copy_from_slice(&(length as u32).to_be_bytes());
    aad[4] = SEALED;
    aad[5..].copy_from_slice(&counter.to_be_bytes());
    (nonce, aad)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonce_budget_exhaustion_closes_both_directions() {
        let mut channel = SealedChannel::new(Secret::new([1; 32]), Secret::new([1; 32]));
        channel.send_counter = COUNTER_LIMIT;
        assert!(channel.seal(0xff, &[]).is_err());
        assert!(channel.is_closed());
        let mut channel = SealedChannel::new(Secret::new([1; 32]), Secret::new([1; 32]));
        channel.receive_counter = COUNTER_LIMIT;
        let mut body = COUNTER_LIMIT.to_be_bytes().to_vec();
        body.extend_from_slice(&[0; 17]);
        assert!(channel.open(&Frame { kind: SEALED, body }).is_err());
        assert!(channel.is_closed());
    }
}
