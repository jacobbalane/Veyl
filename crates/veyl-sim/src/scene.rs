//! Scene parameters describe acoustic paths and movements, never engine feature values.
use serde::{Deserialize, Serialize};
use std::io;
pub mod render;
pub use render::render;

#[derive(Clone)]
pub enum Music {
    Bright { seed: u64 },
    Dull { seed: u64 },
    Compressed16k { seed: u64 },
    Gappy { seed: u64 },
    Silence,
    Samples(Vec<[f32; 2]>),
}
#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Channel {
    Left,
    Right,
    Mix,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Transducer {
    pub position: [f64; 3],
    pub channel: Channel,
}
#[derive(Clone, Copy)]
pub enum Colour {
    Flat,
    HighPass1k,
}
#[derive(Clone)]
pub struct StaticEcho {
    pub extra_path_m: f64,
    pub gain: f32,
    pub colour: Colour,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Poke {
    pub from: [f64; 3],
    pub depth: f64,
    pub turn_seconds: f64,
    pub approach_ms: f64,
    pub hold_ms: f64,
    pub retreat_ms: f64,
}
impl Poke {
    pub fn start(&self) -> f64 {
        self.turn_seconds - (self.approach_ms + self.hold_ms) / 1000.0
    }
    pub fn end(&self) -> f64 {
        self.turn_seconds + self.retreat_ms / 1000.0
    }
}
#[derive(Clone)]
pub enum Mover {
    Hover {
        position: [f64; 3],
        jitter_mm: f64,
    },
    Pokes(Vec<Poke>),
    Wave {
        height: f64,
        amplitude: f64,
        hz: f64,
    },
    Walk {
        distance_range: [f64; 2],
        speed: f64,
    },
    Path(Vec<(f64, [f64; 3])>),
}
fn between(a: [f64; 3], b: [f64; 3], amount: f64) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * amount.clamp(0.0, 1.0))
}
impl Mover {
    pub fn position(&self, seconds: f64) -> [f64; 3] {
        match self {
            Self::Hover {
                position,
                jitter_mm,
            } => {
                let mut p = *position;
                for (axis, value) in p.iter_mut().enumerate() {
                    *value += jitter_mm
                        * 0.001
                        * (seconds * std::f64::consts::TAU / [2.1, 3.3, 4.0][axis]).sin();
                }
                p
            }
            Self::Pokes(pokes) => {
                let mut previous = pokes.first().map_or([0.0, 0.0, 0.2], |poke| poke.from);
                let mut end = 0.0;
                for poke in pokes {
                    let start = poke.start();
                    let close = [poke.from[0], poke.from[1], poke.from[2] - poke.depth];
                    if seconds < start {
                        return between(
                            previous,
                            poke.from,
                            (seconds - end) / (start - end).max(1e-9),
                        );
                    }
                    let reached = poke.turn_seconds - poke.hold_ms / 1000.0;
                    if seconds < reached {
                        return between(
                            poke.from,
                            close,
                            (seconds - start) / (poke.approach_ms / 1000.0),
                        );
                    }
                    if seconds < poke.turn_seconds {
                        return close;
                    }
                    if seconds < poke.end() {
                        return between(
                            close,
                            poke.from,
                            (seconds - poke.turn_seconds) / (poke.retreat_ms / 1000.0),
                        );
                    }
                    previous = poke.from;
                    end = poke.end();
                }
                previous
            }
            Self::Wave {
                height,
                amplitude,
                hz,
            } => [
                amplitude * (std::f64::consts::TAU * hz * seconds).sin(),
                0.0,
                *height,
            ],
            Self::Walk {
                distance_range,
                speed,
            } => {
                let width = distance_range[1] - distance_range[0];
                let phase = (seconds * speed / width).rem_euclid(2.0);
                [
                    0.0,
                    0.0,
                    distance_range[0] + width * if phase < 1.0 { phase } else { 2.0 - phase },
                ]
            }
            Self::Path(points) => {
                let Some(first) = points.first() else {
                    return [0.0, 0.0, 1.0];
                };
                if seconds <= first.0 {
                    return first.1;
                }
                for pair in points.windows(2) {
                    if seconds <= pair[1].0 {
                        return between(
                            pair[0].1,
                            pair[1].1,
                            (seconds - pair[0].0) / (pair[1].0 - pair[0].0),
                        );
                    }
                }
                points.last().unwrap().1
            }
        }
    }
    pub fn gain(&self) -> f64 {
        if matches!(self, Self::Walk { .. }) {
            0.6
        } else {
            0.15
        }
    }
}
#[derive(Clone)]
pub enum RoomNoise {
    Fan,
    Typing { rate_hz: f64 },
    Speech { dbfs: f32 },
}
#[derive(Clone)]
pub struct SpeakerModel {
    pub highpass_hz: f32,
    pub shelf_hz: f32,
    pub shelf_db: f32,
}
impl Default for SpeakerModel {
    fn default() -> Self {
        Self {
            highpass_hz: 300.0,
            shelf_hz: 14000.0,
            shelf_db: -3.0,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SceneEvent {
    Volume { at: f64, gain: f32 },
    Route { at: f64, external: bool },
    CaptureBlocked { at: f64, blocked: bool },
}
impl SceneEvent {
    pub fn at(&self) -> f64 {
        match self {
            Self::Volume { at, .. } | Self::Route { at, .. } | Self::CaptureBlocked { at, .. } => {
                *at
            }
        }
    }
}
pub struct Scene {
    pub fs: u32,
    pub seconds: f32,
    pub music: Music,
    pub volume: f32,
    pub speakers: Vec<Transducer>,
    pub mics: Vec<Transducer>,
    pub statics: Vec<StaticEcho>,
    pub movers: Vec<Mover>,
    pub output_latency_ms: f64,
    pub clock_drift_ppm: f64,
    pub mic_noise_dbfs: f32,
    pub room_noise: Vec<RoomNoise>,
    pub speaker_model: SpeakerModel,
    pub dropouts: Vec<(f64, f64)>,
    pub events: Vec<SceneEvent>,
    pub seed: u64,
}
impl Scene {
    pub fn phone(seed: u64, seconds: f32) -> Self {
        Self {
            fs: 48000,
            seconds,
            seed,
            music: Music::Bright { seed },
            volume: 0.5,
            speakers: vec![Transducer {
                position: [0.02, -0.075, 0.0],
                channel: Channel::Mix,
            }],
            mics: vec![
                Transducer {
                    position: [-0.02, -0.075, 0.0],
                    channel: Channel::Mix,
                },
                Transducer {
                    position: [0.01, 0.075, 0.0],
                    channel: Channel::Mix,
                },
            ],
            statics: vec![
                StaticEcho {
                    extra_path_m: 0.03,
                    gain: 0.3,
                    colour: Colour::Flat,
                },
                StaticEcho {
                    extra_path_m: 0.8,
                    gain: 0.05,
                    colour: Colour::Flat,
                },
            ],
            movers: vec![],
            output_latency_ms: 40.0,
            clock_drift_ppm: 0.0,
            mic_noise_dbfs: -85.0,
            room_noise: vec![],
            speaker_model: SpeakerModel::default(),
            dropouts: vec![],
            events: vec![],
        }
    }
    pub fn validate(&self) -> io::Result<()> {
        let finite = |point: &[f64; 3]| point.iter().all(|v| v.is_finite() && v.abs() <= 100.0);
        if self.fs != 48000 || self.seed == 0 || !self.seconds.is_finite() || !(0.01..=600.0).contains(&self.seconds) || !(0.0..=1.0).contains(&self.volume)
            || !(1..=2).contains(&self.speakers.len()) || !(1..=4).contains(&self.mics.len()) || self.speakers.iter().chain(&self.mics).any(|p| !finite(&p.position))
            || !self.output_latency_ms.is_finite() || !(0.0..=1000.0).contains(&self.output_latency_ms) || !self.clock_drift_ppm.is_finite() || self.clock_drift_ppm.abs() > 1000.0
            || !self.mic_noise_dbfs.is_finite() || !(-160.0..=0.0).contains(&self.mic_noise_dbfs) || self.statics.len() > 64 || self.movers.len() > 16 || self.room_noise.len() > 16
            || self.statics.iter().any(|echo| !echo.extra_path_m.is_finite() || !(0.0..=30.0).contains(&echo.extra_path_m) || !echo.gain.is_finite() || echo.gain.abs() > 4.0)
            || self.dropouts.iter().any(|&(at, seconds)| !at.is_finite() || !seconds.is_finite() || at < 0.0 || seconds < 0.0)
            || self.events.iter().any(|event| !event.at().is_finite() || event.at() < 0.0 || matches!(event, SceneEvent::Volume { gain, .. } if !gain.is_finite() || !(0.0..=4.0).contains(gain)))
            || self.events.windows(2).any(|pair| pair[1].at() < pair[0].at()) {
            return Err(invalid());
        }
        for mover in &self.movers {
            let valid = match mover {
                Mover::Hover {
                    position,
                    jitter_mm,
                } => finite(position) && jitter_mm.is_finite() && (0.0..=100.0).contains(jitter_mm),
                Mover::Pokes(pokes) => {
                    pokes.iter().all(|p| {
                        finite(&p.from)
                            && p.depth.is_finite()
                            && p.depth > 0.0
                            && p.depth < p.from[2]
                            && p.turn_seconds.is_finite()
                            && p.start() >= 0.0
                            && p.approach_ms.is_finite()
                            && (1.0..=3000.0).contains(&p.approach_ms)
                            && p.hold_ms.is_finite()
                            && (0.0..=3000.0).contains(&p.hold_ms)
                            && p.retreat_ms.is_finite()
                            && (1.0..=3000.0).contains(&p.retreat_ms)
                    }) && pokes.windows(2).all(|pair| pair[0].end() < pair[1].start())
                }
                Mover::Wave {
                    height,
                    amplitude,
                    hz,
                } => {
                    height.is_finite()
                        && *height > 0.0
                        && amplitude.is_finite()
                        && (0.0..=1.0).contains(amplitude)
                        && hz.is_finite()
                        && (0.0..=10.0).contains(hz)
                }
                Mover::Walk {
                    distance_range,
                    speed,
                } => {
                    distance_range[0].is_finite()
                        && distance_range[1].is_finite()
                        && distance_range[0] > 0.0
                        && distance_range[1] > distance_range[0]
                        && speed.is_finite()
                        && *speed > 0.0
                }
                Mover::Path(points) => {
                    !points.is_empty()
                        && points
                            .iter()
                            .all(|(t, p)| t.is_finite() && *t >= 0.0 && finite(p))
                        && points.windows(2).all(|p| p[1].0 > p[0].0)
                }
            };
            if !valid {
                return Err(invalid());
            }
        }
        if self.room_noise.iter().any(|noise| match noise {
            RoomNoise::Typing { rate_hz } => {
                !rate_hz.is_finite() || !(0.0..=100.0).contains(rate_hz)
            }
            RoomNoise::Speech { dbfs } => !dbfs.is_finite() || !(-160.0..=0.0).contains(dbfs),
            _ => false,
        }) {
            return Err(invalid());
        }
        let eq = &self.speaker_model;
        if !eq.highpass_hz.is_finite()
            || !(20.0..=8000.0).contains(&eq.highpass_hz)
            || !eq.shelf_hz.is_finite()
            || !(1000.0..=20000.0).contains(&eq.shelf_hz)
            || !eq.shelf_db.is_finite()
            || eq.shelf_db.abs() > 24.0
        {
            return Err(invalid());
        }
        if let Music::Samples(samples) = &self.music {
            if samples.is_empty()
                || samples.len() > 48000 * 600
                || samples
                    .iter()
                    .flatten()
                    .any(|x| !x.is_finite() || x.abs() > 1.0)
            {
                return Err(invalid());
            }
        }
        if matches!(
            &self.music,
            Music::Bright { seed: 0 }
                | Music::Dull { seed: 0 }
                | Music::Compressed16k { seed: 0 }
                | Music::Gappy { seed: 0 }
        ) {
            return Err(invalid());
        }
        Ok(())
    }
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "Invalid acoustic scene.")
}
pub(crate) fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt()
}
