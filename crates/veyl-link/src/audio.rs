use crate::{invalid, AUDIO};
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioBlock {
    pub stream: u8,
    pub channels: u8,
    pub sample_rate: u32,
    pub seq: u32,
    pub first_frame: u64,
    pub capture_ns: i64,
    pub frames: u16,
    pub flags: u16,
    pub pre_rms_dbfs: f32,
    pub pre_peak: f32,
    pub samples: Vec<i16>,
}
impl AudioBlock {
    fn validate(&self) -> io::Result<()> {
        if self.stream > 1
            || !(1..=4).contains(&self.channels)
            || ![44100, 48000].contains(&self.sample_rate)
            || !(1..=4096).contains(&self.frames)
            || self.flags & !15 != 0
            || !self.pre_rms_dbfs.is_finite()
            || !self.pre_peak.is_finite()
            || self.samples.len() != self.frames as usize * self.channels as usize
        {
            return Err(invalid("Invalid AUDIO block."));
        }
        Ok(())
    }
    /// Includes the sealed plaintext's type byte, as in the specification's offsets.
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let mut bytes = vec![AUDIO, self.stream, self.channels, 1];
        bytes.extend_from_slice(&self.sample_rate.to_le_bytes());
        bytes.extend_from_slice(&self.seq.to_le_bytes());
        bytes.extend_from_slice(&self.first_frame.to_le_bytes());
        bytes.extend_from_slice(&self.capture_ns.to_le_bytes());
        bytes.extend_from_slice(&self.frames.to_le_bytes());
        bytes.extend_from_slice(&self.flags.to_le_bytes());
        bytes.extend_from_slice(&self.pre_rms_dbfs.to_le_bytes());
        bytes.extend_from_slice(&self.pre_peak.to_le_bytes());
        for sample in &self.samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < 40
            || bytes[0] != AUDIO
            || bytes[3] != 1
            || !(bytes.len() - 40).is_multiple_of(2)
        {
            return Err(invalid("Invalid AUDIO header."));
        }
        let channels = bytes[2];
        let frames = u16::from_le_bytes(bytes[28..30].try_into().unwrap());
        if bytes.len() != 40 + frames as usize * channels as usize * 2 {
            return Err(invalid("Invalid AUDIO length."));
        }
        let block = Self {
            stream: bytes[1],
            channels,
            sample_rate: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            seq: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            first_frame: u64::from_le_bytes(bytes[12..20].try_into().unwrap()),
            capture_ns: i64::from_le_bytes(bytes[20..28].try_into().unwrap()),
            frames,
            flags: u16::from_le_bytes(bytes[30..32].try_into().unwrap()),
            pre_rms_dbfs: f32::from_le_bytes(bytes[32..36].try_into().unwrap()),
            pre_peak: f32::from_le_bytes(bytes[36..40].try_into().unwrap()),
            samples: bytes[40..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|sample| i16::from_le_bytes(*sample))
                .collect(),
        };
        block.validate()?;
        Ok(block)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ping(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pong {
    pub pc_ns: u64,
    pub phone_ns: i64,
}
impl Ping {
    pub fn encode(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        Ok(Self(u64::from_le_bytes(
            bytes
                .try_into()
                .map_err(|_| invalid("Invalid PING length."))?,
        )))
    }
}
impl Pong {
    pub fn encode(self) -> Vec<u8> {
        [
            self.pc_ns.to_le_bytes().as_slice(),
            &self.phone_ns.to_le_bytes(),
        ]
        .concat()
    }
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != 16 {
            return Err(invalid("Invalid PONG length."));
        }
        Ok(Self {
            pc_ns: u64::from_le_bytes(bytes[..8].try_into().unwrap()),
            phone_ns: i64::from_le_bytes(bytes[8..].try_into().unwrap()),
        })
    }
}
