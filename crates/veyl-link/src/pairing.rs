//! Pairing owns its ephemeral keys. Dropping or aborting it wipes unconfirmed secrets.
use crate::{
    crypto::{self, Ephemeral, PairKeys, Secret},
    invalid,
    messages::{Bytes, Handshake},
    Frame,
};
use std::{collections::VecDeque, io};

#[derive(Clone)]
pub struct Identity {
    pub id: [u8; 16],
    pub name: String,
    pub model: String,
    pub app: String,
}
#[derive(Clone)]
pub struct Peer {
    pub id: [u8; 16],
    pub name: String,
    pub model: String,
}
pub struct PairedPeer {
    pub peer: Peer,
    pub link: Secret,
}

#[derive(Default)]
pub struct PairingRateLimit {
    attempts: VecDeque<u64>,
    last: Option<u64>,
    blocked_until: u64,
}
impl PairingRateLimit {
    pub fn admit(&mut self, now: u64) -> bool {
        if now < self.blocked_until
            || self
                .last
                .is_some_and(|last| now.saturating_sub(last) < 3000)
        {
            return false;
        }
        while self
            .attempts
            .front()
            .is_some_and(|time| now.saturating_sub(*time) >= 600_000)
        {
            self.attempts.pop_front();
        }
        if self.attempts.len() >= 10 {
            self.blocked_until = now.saturating_add(600_000);
            return false;
        }
        self.last = Some(now);
        self.attempts.push_back(now);
        true
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Hello,
    Reply,
    Reveal,
    Confirm,
    Done,
    Failed,
}

pub struct Pairing {
    pc: bool,
    identity: Identity,
    phase: Phase,
    ephemeral: Option<Ephemeral>,
    bodies: Vec<Vec<u8>>,
    commitment: Option<[u8; 32]>,
    peer: Option<Peer>,
    keys: Option<PairKeys>,
    own_confirmed: bool,
    peer_confirmed: bool,
    started: u64,
    confirmation_started: Option<u64>,
    failure: Option<String>,
    denial: Option<&'static str>,
    paired: Option<PairedPeer>,
    sas: Option<String>,
}
impl Pairing {
    fn new(pc: bool, identity: Identity, now: u64) -> Self {
        Self {
            pc,
            identity,
            phase: if pc { Phase::Reply } else { Phase::Hello },
            ephemeral: None,
            bodies: vec![],
            commitment: None,
            peer: None,
            keys: None,
            own_confirmed: false,
            peer_confirmed: false,
            started: now,
            confirmation_started: None,
            failure: None,
            denial: None,
            paired: None,
            sas: None,
        }
    }
    pub fn pc(identity: Identity, now: u64) -> io::Result<(Self, Frame)> {
        Self::pc_with_ephemeral(identity, now, Ephemeral::generate()?)
    }
    pub fn pc_with_ephemeral(
        identity: Identity,
        now: u64,
        ephemeral: Ephemeral,
    ) -> io::Result<(Self, Frame)> {
        let mut this = Self::new(true, identity, now);
        let frame = Handshake::PairHello {
            v: 1,
            pc_id: Bytes(this.identity.id),
            pc_name: this.identity.name.clone(),
            app: this.identity.app.clone(),
            commit: Bytes(crypto::commit(&ephemeral.public, &ephemeral.nonce)),
        }
        .frame()?;
        this.bodies.push(frame.body.clone());
        this.ephemeral = Some(ephemeral);
        Ok((this, frame))
    }
    /// Call after receiving a pair_hello, so sessions do not consume pairing attempts.
    pub fn phone(
        identity: Identity,
        now: u64,
        foreground: bool,
        busy: bool,
        limiter: &mut PairingRateLimit,
    ) -> Self {
        let mut this = Self::new(false, identity, now);
        this.denial = if busy {
            Some("busy")
        } else if !foreground {
            Some("not_in_foreground")
        } else if !limiter.admit(now) {
            Some("rate_limited")
        } else {
            None
        };
        this
    }
    pub fn sas(&self) -> Option<&str> {
        if self.phase == Phase::Confirm {
            self.sas.as_deref()
        } else {
            None
        }
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn is_done(&self) -> bool {
        self.phase == Phase::Done
    }
    pub fn take_pairing(&mut self) -> Option<PairedPeer> {
        self.paired.take()
    }
    fn fail(&mut self, reason: &str) {
        self.phase = Phase::Failed;
        self.failure = Some(reason.into());
        self.ephemeral = None;
        self.keys = None;
        self.sas = None;
        self.paired = None;
    }
    fn abort(&mut self, reason: &str) -> io::Result<Vec<Frame>> {
        self.fail(reason);
        Ok(vec![Handshake::PairAbort {
            reason: reason.into(),
        }
        .frame()?])
    }
    pub fn tick(&mut self, now: u64) -> io::Result<Vec<Frame>> {
        if matches!(self.phase, Phase::Done | Phase::Failed) {
            return Ok(vec![]);
        }
        if now.saturating_sub(self.started) >= 150_000
            || self
                .confirmation_started
                .is_some_and(|start| now.saturating_sub(start) >= 120_000)
        {
            return self.abort("timeout");
        }
        Ok(vec![])
    }
    pub fn receive(&mut self, frame: &Frame, now: u64) -> io::Result<Vec<Frame>> {
        let expired = self.tick(now)?;
        if !expired.is_empty() {
            return Ok(expired);
        }
        if matches!(self.phase, Phase::Done | Phase::Failed) {
            return Err(invalid("Pairing is closed."));
        }
        let result = self.receive_inner(frame, now);
        if result.is_err() {
            self.fail("bad_message");
        }
        result
    }
    fn receive_inner(&mut self, frame: &Frame, now: u64) -> io::Result<Vec<Frame>> {
        let message = Handshake::parse(frame)?;
        if let Handshake::PairAbort { reason } = message {
            self.fail(&reason);
            return Ok(vec![]);
        }
        if let Handshake::Error { code, .. } = message {
            self.fail(&code);
            return Ok(vec![]);
        }
        match (self.phase, message) {
            (
                Phase::Hello,
                Handshake::PairHello {
                    v,
                    pc_id,
                    pc_name,
                    commit,
                    ..
                },
            ) => {
                if v != 1 {
                    self.fail("version");
                    return Ok(vec![Handshake::error("version")]);
                }
                if let Some(reason) = self.denial {
                    return self.abort(reason);
                }
                self.peer = Some(Peer {
                    id: pc_id.0,
                    name: pc_name,
                    model: String::new(),
                });
                self.commitment = Some(commit.0);
                self.bodies.push(frame.body.clone());
                let ephemeral = Ephemeral::generate()?;
                let reply = Handshake::PairReply {
                    v: 1,
                    phone_id: Bytes(self.identity.id),
                    phone_name: self.identity.name.clone(),
                    model: self.identity.model.clone(),
                    app: self.identity.app.clone(),
                    e_pub: Bytes(ephemeral.public),
                    nonce: Bytes(ephemeral.nonce),
                }
                .frame()?;
                self.ephemeral = Some(ephemeral);
                self.bodies.push(reply.body.clone());
                self.phase = Phase::Reveal;
                Ok(vec![reply])
            }
            (
                Phase::Reply,
                Handshake::PairReply {
                    v,
                    phone_id,
                    phone_name,
                    model,
                    e_pub,
                    ..
                },
            ) => {
                if v != 1 {
                    self.fail("version");
                    return Ok(vec![Handshake::error("version")]);
                }
                self.peer = Some(Peer {
                    id: phone_id.0,
                    name: phone_name,
                    model,
                });
                self.bodies.push(frame.body.clone());
                let ephemeral = self.ephemeral.as_ref().unwrap();
                let reveal = Handshake::PairReveal {
                    e_pub: Bytes(ephemeral.public),
                    nonce: Bytes(ephemeral.nonce),
                }
                .frame()?;
                self.bodies.push(reveal.body.clone());
                self.derive(&e_pub.0, now)?;
                Ok(vec![reveal])
            }
            (Phase::Reveal, Handshake::PairReveal { e_pub, nonce }) => {
                if !crypto::matches(
                    &self.commitment.unwrap(),
                    &crypto::commit(&e_pub.0, &nonce.0),
                ) {
                    return self.abort("mismatch");
                }
                self.bodies.push(frame.body.clone());
                self.derive(&e_pub.0, now)?;
                Ok(vec![])
            }
            (Phase::Confirm, Handshake::PairConfirm { mac }) => {
                if self.peer_confirmed
                    || !crypto::matches(&self.keys.as_ref().unwrap().confirmation(!self.pc), &mac.0)
                {
                    return self.abort("bad_mac");
                }
                self.peer_confirmed = true;
                self.finish();
                Ok(vec![])
            }
            _ => Err(invalid("Unexpected pairing message.")),
        }
    }
    fn derive(&mut self, peer: &[u8; 32], now: u64) -> io::Result<()> {
        let ephemeral = self.ephemeral.take().unwrap();
        let shared = ephemeral.shared(peer)?;
        let keys = PairKeys::derive(
            &shared,
            &self.bodies.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        );
        self.sas = Some(keys.sas());
        self.keys = Some(keys);
        self.phase = Phase::Confirm;
        self.confirmation_started = Some(now);
        Ok(())
    }
    pub fn confirm(&mut self, matches: bool, now: u64) -> io::Result<Vec<Frame>> {
        let expired = self.tick(now)?;
        if !expired.is_empty() {
            return Ok(expired);
        }
        if self.phase != Phase::Confirm || self.own_confirmed {
            return Err(invalid("Pairing is not awaiting confirmation."));
        }
        if !matches {
            return self.abort("user_cancelled");
        }
        let frame = Handshake::PairConfirm {
            mac: Bytes(self.keys.as_ref().unwrap().confirmation(self.pc)),
        }
        .frame()?;
        self.own_confirmed = true;
        self.finish();
        Ok(vec![frame])
    }
    pub fn cancel(&mut self) -> io::Result<Vec<Frame>> {
        if matches!(self.phase, Phase::Done | Phase::Failed) {
            return Err(invalid("Pairing is closed."));
        }
        self.abort("user_cancelled")
    }
    fn finish(&mut self) {
        if self.own_confirmed && self.peer_confirmed {
            let keys = self.keys.take().unwrap();
            self.paired = Some(PairedPeer {
                peer: self.peer.take().unwrap(),
                link: keys.link,
            });
            self.sas = None;
            self.phase = Phase::Done;
        }
    }
}
