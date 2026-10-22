use crate::{invalid, Frame, AUDIO, CONTROL, HANDSHAKE, MAX_CONTROL, MAX_HANDSHAKE, PING, PONG};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io;

#[derive(Clone, PartialEq, Eq)]
pub struct Bytes<const N: usize>(pub [u8; N]);
impl<const N: usize> Serialize for Bytes<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&URL_SAFE_NO_PAD.encode(self.0))
    }
}
impl<'de, const N: usize> Deserialize<'de> for Bytes<N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let bytes = URL_SAFE_NO_PAD
            .decode(&text)
            .map_err(serde::de::Error::custom)?;
        if URL_SAFE_NO_PAD.encode(&bytes) != text {
            return Err(serde::de::Error::custom("Noncanonical base64url."));
        }
        Ok(Self(bytes.try_into().map_err(|_| {
            serde::de::Error::custom("Wrong binary field length.")
        })?))
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Handshake {
    PairHello {
        v: u64,
        pc_id: Bytes<16>,
        pc_name: String,
        app: String,
        commit: Bytes<32>,
    },
    PairReply {
        v: u64,
        phone_id: Bytes<16>,
        phone_name: String,
        model: String,
        app: String,
        e_pub: Bytes<32>,
        nonce: Bytes<32>,
    },
    PairReveal {
        e_pub: Bytes<32>,
        nonce: Bytes<32>,
    },
    PairConfirm {
        mac: Bytes<32>,
    },
    PairAbort {
        reason: String,
    },
    Hello {
        v: u64,
        pc_id: Bytes<16>,
        app: String,
        e_pub: Bytes<32>,
        nonce: Bytes<32>,
    },
    HelloReply {
        v: u64,
        phone_id: Bytes<16>,
        app: String,
        e_pub: Bytes<32>,
        nonce: Bytes<32>,
    },
    Error {
        code: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        supported: Option<Vec<u64>>,
    },
}
impl Handshake {
    pub fn frame(&self) -> io::Result<Frame> {
        let body = serde_json::to_vec(self).map_err(|_| invalid("Invalid handshake JSON."))?;
        let frame = Frame {
            kind: HANDSHAKE,
            body,
        };
        Self::parse(&frame)?;
        Ok(frame)
    }
    pub fn parse(frame: &Frame) -> io::Result<Self> {
        if frame.kind != HANDSHAKE || frame.body.len() > MAX_HANDSHAKE {
            return Err(invalid("Expected bounded handshake JSON."));
        }
        let message: Self =
            serde_json::from_slice(&frame.body).map_err(|_| invalid("Invalid handshake JSON."))?;
        match &message {
            Self::PairHello { pc_name, .. } if pc_name.len() > 40 => {
                return Err(invalid("PC name exceeds 40 UTF-8 bytes."))
            }
            Self::PairReply {
                phone_name, model, ..
            } if phone_name.len() > 40 || model.len() > 40 => {
                return Err(invalid("Phone name or model exceeds 40 UTF-8 bytes."))
            }
            _ => {}
        }
        Ok(message)
    }
    pub fn error(code: &str) -> Frame {
        Self::Error {
            code: code.into(),
            message: None,
            supported: if code == "version" {
                Some(vec![1])
            } else {
                None
            },
        }
        .frame()
        .expect("bounded error message")
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Device {
    pub manufacturer: String,
    pub model: String,
    pub android_sdk: u32,
    pub app_version: String,
    pub phone_name: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Capture {
    pub state: String,
    pub source: String,
    pub route: String,
    pub reason: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Pc {
    pub pc_name: String,
    pub app_version: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Want {
    pub meter: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Ready {
    Phone { device: Device, capture: Capture },
    Pc { pc: Pc, want: Want },
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Microphone {
    pub index: u32,
    #[serde(rename = "type")]
    pub kind: String,
    pub position_m: Option<[f32; 3]>,
    pub channels: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SoundCheck {
    pub mic_source: String,
    pub mic_channels: u8,
    pub score: f32,
    pub mean_coherence: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub run_id: u64,
    pub source: String,
    pub sample_rate: u32,
    pub frames_per_block: u16,
    pub ref_channels: u8,
    pub mic_channels: u8,
    pub mic_source: String,
    pub effects_off: Vec<String>,
    pub hp_cutoff_hz: u32,
    pub mics: Vec<Microphone>,
    pub sound_check: Option<SoundCheck>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Control {
    Ready {
        #[serde(flatten)]
        ready: Ready,
    },
    AudioConfig {
        #[serde(flatten)]
        config: AudioConfig,
    },
    CaptureState {
        #[serde(flatten)]
        capture: Capture,
    },
    Stats {
        queue_ms: u32,
        dropped_blocks: u64,
        media_volume: f32,
        battery_pct: u8,
        charging: bool,
        thermal: String,
        cpu_pct: f32,
    },
    PhoneInfo {
        phone_name: String,
    },
    Meter {
        score: u8,
        label: String,
        tip: String,
    },
    Gesture {
        kind: String,
        acted: bool,
    },
    RequestStart,
    Error {
        code: String,
        message: String,
    },
    Bye {
        reason: String,
    },
    #[serde(other)]
    Unknown,
}
impl Control {
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let bytes = serde_json::to_vec(self).map_err(|_| invalid("Invalid CONTROL JSON."))?;
        if bytes.len() > MAX_CONTROL {
            return Err(invalid("CONTROL size limit."));
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > MAX_CONTROL {
            return Err(invalid("CONTROL size limit."));
        }
        serde_json::from_slice(bytes).map_err(|_| invalid("Invalid CONTROL JSON."))
    }
}
pub(crate) fn validate_payload(kind: u8, payload: &[u8]) -> io::Result<()> {
    match kind {
        CONTROL => {
            Control::decode(payload)?;
        }
        AUDIO => {
            let mut bytes = vec![AUDIO];
            bytes.extend_from_slice(payload);
            crate::audio::AudioBlock::decode(&bytes)?;
        }
        PING => {
            crate::audio::Ping::decode(payload)?;
        }
        PONG => {
            crate::audio::Pong::decode(payload)?;
        }
        _ => {} // IMU and future sealed types are ignored by older peers.
    }
    Ok(())
}
