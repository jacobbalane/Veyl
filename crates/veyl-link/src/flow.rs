//! Network timing and the audio-only loss policy are independent of socket scheduling.
use crate::{audio::AudioBlock, invalid, Frame, CONTROL, PING, PONG};
use std::{collections::VecDeque, io};

pub enum Outbound {
    Audio(AudioBlock),
    Message { kind: u8, payload: Vec<u8> },
}
#[derive(Default)]
pub struct WriterQueue {
    queue: VecDeque<Outbound>,
}
impl WriterQueue {
    pub fn push_audio(&mut self, mut block: AudioBlock) -> io::Result<bool> {
        block.encode()?;
        let stream = block.stream;
        let full = self
            .queue
            .iter()
            .filter(|message| matches!(message, Outbound::Audio(audio) if audio.stream == stream))
            .count()
            >= 100;
        if full {
            let index = self
                .queue
                .iter()
                .position(
                    |message| matches!(message, Outbound::Audio(audio) if audio.stream == stream),
                )
                .unwrap();
            self.queue.remove(index);
            if let Some(Outbound::Audio(next)) = self
                .queue
                .iter_mut()
                .find(|message| matches!(message, Outbound::Audio(audio) if audio.stream == stream))
            {
                next.flags |= 1;
            } else {
                block.flags |= 1;
            }
        }
        self.queue.push_back(Outbound::Audio(block));
        Ok(full)
    }
    pub fn push_message(&mut self, kind: u8, payload: Vec<u8>) -> io::Result<()> {
        if ![CONTROL, PING, PONG].contains(&kind) {
            return Err(invalid("Not a priority message."));
        }
        crate::messages::validate_payload(kind, &payload)?;
        self.queue.push_back(Outbound::Message { kind, payload });
        Ok(())
    }
    pub fn pop(&mut self) -> Option<Outbound> {
        self.queue.pop_front()
    }
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
impl Outbound {
    pub fn seal(self, channel: &mut crate::crypto::SealedChannel) -> io::Result<Frame> {
        match self {
            Self::Audio(block) => channel.seal(crate::AUDIO, &block.encode()?[1..]),
            Self::Message { kind, payload } => channel.seal(kind, &payload),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Heartbeat {
    Idle,
    Ping,
    Close,
}
pub struct Liveness {
    pc: bool,
    last_receive: u64,
    last_ping: u64,
}
impl Liveness {
    pub fn new(pc: bool, now: u64) -> Self {
        Self {
            pc,
            last_receive: now,
            last_ping: now,
        }
    }
    pub fn received(&mut self, now: u64) {
        self.last_receive = now;
    }
    pub fn tick(&mut self, now: u64) -> Heartbeat {
        if now.saturating_sub(self.last_receive) >= if self.pc { 3000 } else { 10_000 } {
            return Heartbeat::Close;
        }
        if self.pc && now.saturating_sub(self.last_ping) >= 1000 {
            self.last_ping = now;
            return Heartbeat::Ping;
        }
        Heartbeat::Idle
    }
}
#[derive(Default)]
pub struct Reconnect {
    attempt: usize,
    first_failure: Option<u64>,
}
impl Reconnect {
    pub fn failed(&mut self, now: u64) -> u64 {
        self.first_failure.get_or_insert(now);
        let delay = [500, 1000, 2000, 4000, 5000][self.attempt.min(4)];
        self.attempt = self.attempt.saturating_add(1);
        delay
    }
    pub fn unreachable(&self, now: u64) -> bool {
        self.first_failure
            .is_some_and(|start| now.saturating_sub(start) >= 30_000)
    }
    pub fn connected(&mut self) {
        self.attempt = 0;
        self.first_failure = None;
    }
}
