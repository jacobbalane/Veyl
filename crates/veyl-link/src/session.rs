//! Authentication completes only after both sides exchange the first sealed ready message.
use crate::{
    crypto::{Ephemeral, SealedChannel, Secret, SessionKeys},
    invalid,
    messages::{Bytes, Control, Handshake, Ready},
    pairing::{Identity, PairedPeer, Peer},
    Frame, CONTROL,
};
use std::io;

#[derive(PartialEq, Eq)]
enum Phase {
    Hello,
    Reply,
    Ready,
    Done,
    Failed,
}
pub struct Session {
    pc: bool,
    identity: Identity,
    peer: Option<Peer>,
    link: Option<Secret>,
    ephemeral: Option<Ephemeral>,
    hello: Vec<u8>,
    ready: Control,
    channel: Option<SealedChannel>,
    phase: Phase,
    started: u64,
    busy: bool,
}
impl Session {
    fn new(
        pc: bool,
        identity: Identity,
        paired: Option<PairedPeer>,
        ready: Control,
        now: u64,
        busy: bool,
    ) -> io::Result<Self> {
        if !matches!(
            (&ready, pc),
            (
                Control::Ready {
                    ready: Ready::Pc { .. }
                },
                true
            ) | (
                Control::Ready {
                    ready: Ready::Phone { .. }
                },
                false
            )
        ) {
            return Err(invalid("Wrong local ready role."));
        }
        let (peer, link) = paired
            .map(|paired| (Some(paired.peer), Some(paired.link)))
            .unwrap_or((None, None));
        Ok(Self {
            pc,
            identity,
            peer,
            link,
            ephemeral: None,
            hello: vec![],
            ready,
            channel: None,
            phase: if pc { Phase::Reply } else { Phase::Hello },
            started: now,
            busy,
        })
    }
    pub fn pc(
        identity: Identity,
        paired: PairedPeer,
        ready: Control,
        now: u64,
    ) -> io::Result<(Self, Frame)> {
        let mut this = Self::new(true, identity, Some(paired), ready, now, false)?;
        let ephemeral = Ephemeral::generate()?;
        let hello = Handshake::Hello {
            v: 1,
            pc_id: Bytes(this.identity.id),
            app: this.identity.app.clone(),
            e_pub: Bytes(ephemeral.public),
            nonce: Bytes(ephemeral.nonce),
        }
        .frame()?;
        this.hello = hello.body.clone();
        this.ephemeral = Some(ephemeral);
        Ok((this, hello))
    }
    /// The caller looks up pc_id and supplies its pairing; None produces not_paired.
    pub fn phone(
        identity: Identity,
        paired: Option<PairedPeer>,
        ready: Control,
        now: u64,
        busy: bool,
    ) -> io::Result<Self> {
        Self::new(false, identity, paired, ready, now, busy)
    }
    pub fn is_done(&self) -> bool {
        self.phase == Phase::Done
    }
    pub fn is_failed(&self) -> bool {
        self.phase == Phase::Failed
    }
    pub fn take_channel(&mut self) -> Option<SealedChannel> {
        if self.is_done() {
            self.channel.take()
        } else {
            None
        }
    }
    fn close(&mut self) {
        self.phase = Phase::Failed;
        self.channel = None;
        self.ephemeral = None;
        self.link = None;
    }
    pub fn tick(&mut self, now: u64) -> io::Result<()> {
        if !matches!(self.phase, Phase::Done | Phase::Failed)
            && now.saturating_sub(self.started) >= 5000
        {
            self.close();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Session handshake timed out.",
            ));
        }
        Ok(())
    }
    pub fn receive(&mut self, frame: &Frame, now: u64) -> io::Result<Vec<Frame>> {
        self.tick(now)?;
        let result = self.receive_inner(frame);
        if result.is_err() {
            self.close();
        }
        result
    }
    fn receive_inner(&mut self, frame: &Frame) -> io::Result<Vec<Frame>> {
        if self.phase == Phase::Ready {
            let channel = self.channel.as_mut().unwrap();
            let (kind, payload) = channel.open(frame)?;
            if kind != CONTROL
                || !matches!(
                    (Control::decode(&payload)?, self.pc),
                    (
                        Control::Ready {
                            ready: Ready::Phone { .. }
                        },
                        true
                    ) | (
                        Control::Ready {
                            ready: Ready::Pc { .. }
                        },
                        false
                    )
                )
            {
                return Err(invalid("Expected peer ready."));
            }
            let outbound = if self.pc {
                vec![channel.seal(CONTROL, &self.ready.encode()?)?]
            } else {
                vec![]
            };
            self.phase = Phase::Done;
            return Ok(outbound);
        }
        if matches!(self.phase, Phase::Done | Phase::Failed) {
            return Err(invalid("Session handshake is closed."));
        }
        let message = Handshake::parse(frame)?;
        match (self.pc, message) {
            (
                false,
                Handshake::Hello {
                    v, pc_id, e_pub, ..
                },
            ) => {
                let error = if v != 1 {
                    Some("version")
                } else if self.busy {
                    Some("busy")
                } else if self.peer.as_ref().is_none_or(|peer| peer.id != pc_id.0) {
                    Some("not_paired")
                } else {
                    None
                };
                if let Some(code) = error {
                    self.close();
                    return Ok(vec![Handshake::error(code)]);
                }
                let ephemeral = Ephemeral::generate()?;
                let reply = Handshake::HelloReply {
                    v: 1,
                    phone_id: Bytes(self.identity.id),
                    app: self.identity.app.clone(),
                    e_pub: Bytes(ephemeral.public),
                    nonce: Bytes(ephemeral.nonce),
                }
                .frame()?;
                let shared = ephemeral.shared(&e_pub.0)?;
                let keys = SessionKeys::derive(
                    &shared,
                    self.link.as_ref().unwrap(),
                    &[&frame.body, &reply.body],
                );
                let mut channel = keys.channel(false);
                let ready = channel.seal(CONTROL, &self.ready.encode()?)?;
                self.link = None;
                self.channel = Some(channel);
                self.phase = Phase::Ready;
                Ok(vec![reply, ready])
            }
            (
                true,
                Handshake::HelloReply {
                    v, phone_id, e_pub, ..
                },
            ) => {
                if v != 1 {
                    self.close();
                    return Ok(vec![Handshake::error("version")]);
                }
                if self.peer.as_ref().unwrap().id != phone_id.0 {
                    return Err(invalid("Unexpected phone identity."));
                }
                let ephemeral = self.ephemeral.take().unwrap();
                let shared = ephemeral.shared(&e_pub.0)?;
                self.channel = Some(
                    SessionKeys::derive(
                        &shared,
                        self.link.as_ref().unwrap(),
                        &[&self.hello, &frame.body],
                    )
                    .channel(true),
                );
                self.link = None;
                self.phase = Phase::Ready;
                Ok(vec![])
            }
            (_, Handshake::Error { .. }) => Err(invalid("Peer rejected session.")),
            _ => Err(invalid("Unexpected session handshake message.")),
        }
    }
}

/// Clock conversion affects diagnostics, never stream alignment.
#[derive(Default)]
pub struct ClockSync {
    samples: std::collections::VecDeque<(u64, i128)>,
}
impl ClockSync {
    pub fn observe(&mut self, sent_ns: u64, received_ns: u64, phone_ns: i64) -> Option<i128> {
        let rtt = received_ns.checked_sub(sent_ns)?;
        self.samples
            .push_back((rtt, phone_ns as i128 - (sent_ns as i128 + rtt as i128 / 2)));
        if self.samples.len() > 15 {
            self.samples.pop_front();
        }
        let mut rtts: Vec<_> = self.samples.iter().map(|sample| sample.0).collect();
        rtts.sort_unstable();
        let median_rtt = rtts[rtts.len() / 2];
        let mut offsets: Vec<_> = self
            .samples
            .iter()
            .filter(|sample| sample.0 <= median_rtt)
            .map(|sample| sample.1)
            .collect();
        offsets.sort_unstable();
        Some(offsets[offsets.len() / 2])
    }
}
