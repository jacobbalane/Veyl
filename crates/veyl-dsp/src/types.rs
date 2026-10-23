//! Source-independent, borrowed input. The engine does not own a socket or capture device.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    Low,
    #[default]
    Normal,
    High,
}
impl Sensitivity {
    pub fn z_on(self) -> f32 {
        match self {
            Self::Low => 12.0,
            Self::Normal => 8.0,
            Self::High => 5.0,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RangePreset {
    Near,
    #[default]
    Normal,
    Far,
}
impl RangePreset {
    pub fn limits_cm(self) -> (f32, f32) {
        (
            4.0,
            match self {
                Self::Near => 25.0,
                Self::Normal => 40.0,
                Self::Far => 55.0,
            },
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StreamFormat {
    pub sample_rate: u32,
    pub ref_channels: u8,
    pub mic_channels: u8,
}
impl StreamFormat {
    pub fn validate(self) -> Result<Self, &'static str> {
        if ![44100, 48000].contains(&self.sample_rate)
            || !(1..=2).contains(&self.ref_channels)
            || !(1..=4).contains(&self.mic_channels)
        {
            return Err("Unsupported engine stream format.");
        }
        Ok(self)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Reference,
    Microphone,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct BlockFlags(pub u16);
impl BlockFlags {
    pub const GAP_BEFORE: u16 = 1;
    pub const CLIPPED: u16 = 2;
    pub const TEST_SOUND: u16 = 4;
    pub const SYNTHETIC_REF: u16 = 8;
}
pub struct BlockIn<'a> {
    pub stream: Stream,
    pub first_frame: u64,
    pub capture_ns: Option<i64>,
    pub flags: BlockFlags,
    pub pre_rms_dbfs: f32,
    pub pre_peak: f32,
    pub samples: &'a [i16],
}
