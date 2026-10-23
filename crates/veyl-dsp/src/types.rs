//! Source-independent, borrowed input. The engine does not own a socket or capture device.
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
