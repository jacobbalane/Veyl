use crate::{
    align::{Aligner, LockChange},
    background::Background,
    config::EngineConfig,
    echo::{direct_peak_near, nearest_direct, DirectPeak, EchoEstimator, EchoFrame},
    meter::{Meter, MeterReading, SourceStatus},
    motion::{MotionFeatures, MotionInput, MotionTracker},
    poke::{PokeDetector, PokeEvent},
    resample::FractionalSinc,
    ring::SampleRing,
    BlockFlags, BlockIn, RangePreset, Sensitivity, Stream, StreamFormat,
};
use serde::{Deserialize, Serialize};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineState {
    #[default]
    NoAudio,
    WaitingForSound,
    Aligning,
    Settling,
    Running,
}

#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Warning {
    Relocked,
    ModelJump,
    ImpulsiveNoise,
    Drift,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum EngineEvent {
    State(EngineState),
    Aligned {
        lag_frames: i64,
        lag_ms: f32,
        psr: f32,
    },
    Hand {
        present: bool,
        distance_cm: Option<f32>,
    },
    Features(MotionFeatures),
    Poke(PokeEvent),
    Meter(MeterReading),
    Warning(Warning),
}
#[derive(Clone, Copy, Debug)]
pub struct EngineSnapshot {
    pub state: EngineState,
    pub frame: Option<u64>,
    pub lag_frames: Option<i64>,
    pub drift_ppm: f64,
    pub direct_residual: Option<f64>,
    pub hand: MotionFeatures,
    pub motion: [f32; 60],
    pub echo: Option<EchoFrame>,
    pub meter: MeterReading,
    pub suspended: bool,
    pub paused: bool,
    pub processed_hops: u64,
    pub invalid_hops: u64,
}

/// Pure processing; capture, transport, recording and UI are owned by the caller.
pub struct Engine {
    config: EngineConfig,
    format: StreamFormat,
    reference: SampleRing,
    microphone: SampleRing,
    aligner: Aligner,
    echo: EchoEstimator,
    background: Background,
    motion: MotionTracker,
    detector: PokeDetector,
    meter: Meter,
    reference_window: Vec<f32>,
    reference_scratch: Vec<f32>,
    reference_sampler: FractionalSinc<64>,
    microphone_window: Vec<f32>,
    state: EngineState,
    next_hop: Option<u64>,
    last_frame: Option<u64>,
    latest_echo: Option<EchoFrame>,
    reference_power: f32,
    running_frames: u64,
    suspended_until: u64,
    model_jump_until: u64,
    paused: bool,
    impulsive_noise: bool,
    drift_warning: bool,
    row: Cell<[f32; 60]>,
    processed_hops: u64,
    invalid_hops: u64,
    pending_hand_end: bool,
    jump_streak: u32,
    /// Per microphone: the tracked direct position and where it started.
    residual_direct: Vec<Option<(f64, f64)>>,
    own_directs: Vec<Option<f64>>,
}
/// The strongest tap summed over a microphone's pairs, with a parabolic fraction.
fn strongest_tap(maps: &[rustfft::num_complex::Complex<f32>], length: usize) -> Option<f64> {
    let at = |i: usize| {
        maps.chunks(length)
            .map(|pair| pair[i].norm_sqr())
            .sum::<f32>()
    };
    let index = (0..length).max_by(|a, b| at(*a).total_cmp(&at(*b)))?;
    let centre = at(index);
    if centre <= 1e-20 {
        return None;
    }
    let left = at(index.saturating_sub(1));
    let right = at((index + 1).min(length - 1));
    let curvature = 2.0 * centre - left - right;
    let fraction = if curvature > 1e-20 {
        (0.5 * (right - left) / curvature).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    Some(index as f64 + fraction as f64)
}
impl Engine {
    pub fn new(config: EngineConfig, format: StreamFormat) -> Result<Self, &'static str> {
        config.validate()?;
        format.validate()?;
        Ok(Self {
            reference: SampleRing::new(
                format.sample_rate,
                format.ref_channels,
                config.ring_seconds,
            )?,
            microphone: SampleRing::new(
                format.sample_rate,
                format.mic_channels,
                config.ring_seconds,
            )?,
            aligner: Aligner::new(config.clone(), format, config.pc_mode)?,
            echo: EchoEstimator::new(config.clone(), format)?,
            background: Background::new(config.clone(), format)?,
            motion: MotionTracker::new(config.clone(), format)?,
            detector: PokeDetector::new(config.clone(), format.sample_rate)?,
            meter: Meter::new(
                format.sample_rate,
                config.hop,
                config.pc_mode,
                config.temperature_c,
            ),
            reference_window: vec![0.0; config.window * format.ref_channels as usize],
            reference_scratch: vec![0.0; config.window * 2 + 128],
            reference_sampler: FractionalSinc::new()?,
            microphone_window: vec![0.0; config.window * format.mic_channels as usize],
            state: EngineState::NoAudio,
            next_hop: None,
            last_frame: None,
            latest_echo: None,
            reference_power: 0.0,
            running_frames: 0,
            suspended_until: 0,
            model_jump_until: 0,
            paused: false,
            impulsive_noise: false,
            drift_warning: false,
            row: Cell::new([0.0; 60]),
            processed_hops: 0,
            invalid_hops: 0,
            pending_hand_end: false,
            jump_streak: 0,
            residual_direct: vec![None; format.mic_channels as usize],
            own_directs: vec![None; format.mic_channels as usize],
            config,
            format,
        })
    }
    pub fn push(&mut self, block: BlockIn<'_>) -> Result<(), &'static str> {
        let channels = match block.stream {
            Stream::Reference => self.format.ref_channels,
            Stream::Microphone => self.format.mic_channels,
        } as usize;
        if block.samples.len() != self.format.sample_rate as usize / 100 * channels
            || block.flags.0 & !15 != 0
            || !block.pre_rms_dbfs.is_finite()
            || !block.pre_peak.is_finite()
            || !(0.0..=1.0).contains(&block.pre_peak)
        {
            return Err("Invalid engine audio block.");
        }
        let ring = match block.stream {
            Stream::Reference => &mut self.reference,
            Stream::Microphone => &mut self.microphone,
        };
        let accepted = ring.push(
            block.first_frame,
            block.samples,
            block.capture_ns,
            block.flags.0 & BlockFlags::GAP_BEFORE != 0,
        )?;
        if accepted.frames == block.samples.len() / channels {
            self.meter.observe_block(&block, channels);
        }
        if accepted.frames > 0 && block.stream == Stream::Reference {
            let power = block
                .samples
                .iter()
                .map(|sample| (*sample as f32 / 32768.0).powi(2))
                .sum::<f32>()
                / block.samples.len() as f32;
            let alpha = 1.0 - (-0.01_f32 / 0.1).exp();
            self.reference_power += alpha * (power - self.reference_power);
        }
        Ok(())
    }
    fn set_state(&mut self, state: EngineState, events: &mut Vec<EngineEvent>) {
        if self.state != state {
            self.state = state;
            events.push(EngineEvent::State(state));
        }
    }
    fn reset_motion(&mut self, short: bool, events: &mut Vec<EngineEvent>) {
        if self.motion.present() || self.pending_hand_end {
            events.push(EngineEvent::Hand {
                present: false,
                distance_cm: None,
            });
        }
        self.pending_hand_end = false;
        self.residual_direct.fill(None);
        self.background.reset(short);
        self.motion.reset();
        self.detector.suspend();
        self.running_frames = 0;
        self.row.set([0.0; 60]);
    }
    pub fn set_sensitivity(&mut self, preset: Sensitivity) {
        self.pending_hand_end |= self.motion.present();
        self.motion.set_sensitivity(preset);
        self.detector.set_sensitivity(preset);
    }
    pub fn set_range(&mut self, preset: RangePreset) {
        self.pending_hand_end |= self.motion.present();
        self.motion.set_range(preset);
        self.detector.set_range(preset);
    }
    pub fn set_paused(&mut self, paused: bool) {
        if self.paused != paused {
            self.detector.suspend();
        }
        self.paused = paused;
    }
    pub fn set_source_status(&mut self, status: SourceStatus) -> Result<(), &'static str> {
        // Volume changes use direct-path normalization; the capture context has not changed.
        let changed = SourceStatus {
            media_volume: status.media_volume,
            ..self.meter.status()
        } != status;
        self.meter.set_status(status)?;
        if changed {
            self.detector.suspend();
        }
        Ok(())
    }
    pub fn reset(&mut self) {
        self.reference.clear();
        self.microphone.clear();
        self.aligner.reset();
        self.echo.reset();
        self.background.reset(false);
        self.motion.reset();
        self.detector.reset();
        self.meter = Meter::new(
            self.format.sample_rate,
            self.config.hop,
            self.config.pc_mode,
            self.config.temperature_c,
        );
        self.row.set([0.0; 60]);
        self.state = EngineState::NoAudio;
        self.next_hop = None;
        self.last_frame = None;
        self.latest_echo = None;
        self.reference_power = 0.0;
        self.running_frames = 0;
        self.suspended_until = 0;
        self.model_jump_until = 0;
        self.impulsive_noise = false;
        self.drift_warning = false;
        self.processed_hops = 0;
        self.invalid_hops = 0;
        self.jump_streak = 0;
        self.residual_direct.fill(None);
        self.pending_hand_end = false;
    }
    pub fn snapshot(&self) -> EngineSnapshot {
        EngineSnapshot {
            state: self.state,
            frame: self.last_frame,
            lag_frames: self.aligner.locked().then(|| self.aligner.lag()),
            drift_ppm: self.aligner.drift_ppm(),
            direct_residual: self.aligner.direct_residual(),
            hand: self.motion.features(),
            motion: self.row.replace([0.0; 60]),
            echo: self.latest_echo,
            meter: self.meter.reading(),
            suspended: self.paused
                || !self.meter.status().permits_detection(self.config.pc_mode)
                || self.meter.reading().score < 15
                || !self.meter.reference_active()
                || self.state != EngineState::Running
                || self
                    .last_frame
                    .is_none_or(|frame| frame < self.suspended_until),
            paused: self.paused,
            processed_hops: self.processed_hops,
            invalid_hops: self.invalid_hops,
        }
    }
    /// Correlated channels leave each speaker's share of the fast estimate ambiguous, and several
    /// direct sounds compete for the strongest peak of a summed map. Each microphone's slowly
    /// learned background, seen through the mean reference, keeps a stable direct peak; the hop's whole-field delay against that
    /// background adds the fast change. The feedback averages each microphone's movement from
    /// where it started.
    fn observe_residual_direct(&mut self, mode_changed: bool) {
        let length = (self.config.pre_taps + self.config.taps) * self.format.ref_channels as usize;
        let mut restarted = mode_changed;
        let (mut offset, mut variance, mut energy, mut count) = (0.0, 0.0, 0.0, 0);
        let delays = self.background.field_delays();
        for (mic, maps) in self.background.mean_maps().chunks(length).enumerate() {
            let Some(delay) = delays[mic] else {
                continue;
            };
            let tracked = &mut self.residual_direct[mic];
            let peak = direct_peak_near(
                maps,
                self.config.pre_taps,
                self.config.taps,
                tracked.map(|(position, _)| position),
                self.config.align_tolerance_frames,
                self.config.align_peak_ratio,
            );
            if peak.energy <= 1e-12 || !peak.position.is_finite() {
                continue;
            }
            let position = peak.position + delay;
            let origin = match *tracked {
                Some((_, origin)) => origin,
                None => {
                    restarted = true;
                    position
                }
            };
            *tracked = Some((peak.position, origin));
            offset += position - origin;
            // Each hop's delay fit carries a few hundredths of a frame of noise.
            variance += peak.variance + 0.02;
            energy += peak.energy;
            count += 1;
        }
        if count > 0 {
            self.aligner.observe_fast(
                DirectPeak {
                    position: offset / count as f64,
                    variance: variance / (count * count) as f64,
                    energy,
                },
                restarted,
            );
        }
    }
    /// The caller can reuse an event Vec; this method never clears its existing entries.
    pub fn process(&mut self, events: &mut Vec<EngineEvent>) -> Result<(), &'static str> {
        self.process_hops(events)?;
        if let Some(reading) = self.meter.tick(
            self.microphone.end().unwrap_or(0),
            self.reference.end().is_some() && self.microphone.end().is_some(),
            self.impulsive_noise,
        ) {
            events.push(EngineEvent::Meter(reading));
        }
        Ok(())
    }
    fn process_hops(&mut self, events: &mut Vec<EngineEvent>) -> Result<(), &'static str> {
        if self.pending_hand_end {
            events.push(EngineEvent::Hand {
                present: false,
                distance_cm: None,
            });
            self.pending_hand_end = false;
        }
        let (Some(_), Some(mic_end)) = (self.reference.end(), self.microphone.end()) else {
            self.set_state(EngineState::NoAudio, events);
            return Ok(());
        };
        let minimum_power = 10.0_f32.powf(self.config.align_min_dbfs / 10.0);
        if self.reference_power < minimum_power {
            if matches!(self.state, EngineState::Running | EngineState::Settling) {
                self.echo.reset_averages();
                self.reset_motion(true, events);
            }
            self.set_state(EngineState::WaitingForSound, events);
            if self.aligner.locked() {
                self.next_hop = Some(mic_end - 1);
            }
            return Ok(());
        }
        if let Some(change) = self.aligner.try_align(&self.reference, &self.microphone) {
            let estimate = match change {
                LockChange::Locked(value) | LockChange::Relocked(value) => value,
            };
            let relocked = matches!(change, LockChange::Relocked(_));
            self.echo.reset();
            self.reset_motion(relocked, events);
            self.next_hop = Some(mic_end - 1);
            self.suspended_until = mic_end.saturating_add(
                (self.config.jump_suspend_ms * self.format.sample_rate as f32 / 1000.0) as u64,
            );
            if relocked {
                events.push(EngineEvent::Warning(Warning::Relocked));
            }
            events.push(EngineEvent::Aligned {
                lag_frames: estimate.lag_frames,
                lag_ms: estimate.fractional_lag as f32 * 1000.0 / self.format.sample_rate as f32,
                psr: estimate.psr,
            });
        }
        if !self.aligner.locked() {
            self.set_state(EngineState::Aligning, events);
            return Ok(());
        }
        while let Some(frame) = self.next_hop.filter(|frame| *frame < mic_end) {
            let Some((lag, _)) = self.aligner.preview_hop(frame) else {
                break;
            };
            let Some(mic_first) = frame
                .checked_add(1)
                .and_then(|end| end.checked_sub(self.config.window as u64))
            else {
                break;
            };
            let Ok(ref_first) = u64::try_from(mic_first as i128 - lag as i128) else {
                break;
            };
            let rate = self.aligner.drift_ppm() * 1e-6;
            let required_padding = if self.config.resample_reference {
                33.0 + (rate.abs() * self.config.window as f64).ceil()
            } else {
                0.0
            };
            let supported = required_padding.is_finite()
                && required_padding <= (self.config.window / 2 + 64) as f64;
            let padding = if supported {
                required_padding as usize
            } else {
                0
            };
            if ref_first
                .checked_add(self.config.window as u64)
                .and_then(|end| end.checked_add(padding as u64))
                .is_none_or(|end| end > self.reference.end().unwrap())
            {
                break;
            }
            let (_, fraction) = self
                .aligner
                .begin_hop(frame)
                .ok_or("Non-increasing engine hop.")?;
            let mut valid = supported;
            for channel in 0..self.format.ref_channels as usize {
                let output = &mut self.reference_window
                    [channel * self.config.window..(channel + 1) * self.config.window];
                if self.config.resample_reference && supported {
                    let scratch = &mut self.reference_scratch[..self.config.window + 2 * padding];
                    if let Some(first) = ref_first.checked_sub(padding as u64) {
                        valid &= self.reference.copy(first, Some(channel), scratch);
                        for (index, sample) in output.iter_mut().enumerate() {
                            // The tracker predicts delay at this hop's last microphone frame.
                            let at = padding as f64 + index as f64
                                - fraction
                                - rate * (index as f64 - (self.config.window - 1) as f64);
                            *sample = self.reference_sampler.read(scratch, at);
                        }
                    } else {
                        valid = false;
                        output.fill(0.0);
                    }
                } else {
                    valid &= self.reference.copy(ref_first, Some(channel), output);
                }
            }
            for channel in 0..self.format.mic_channels as usize {
                valid &= self.microphone.copy(
                    mic_first,
                    Some(channel),
                    &mut self.microphone_window
                        [channel * self.config.window..(channel + 1) * self.config.window],
                );
            }
            self.last_frame = Some(frame);
            self.processed_hops += 1;
            self.next_hop = frame.checked_add(self.config.hop as u64);
            if !valid {
                self.invalid_hops += 1;
                self.suspended_until = frame.saturating_add(
                    (self.config.jump_suspend_ms * self.format.sample_rate as f32 / 1000.0) as u64,
                );
                let features = self.motion.process(MotionInput {
                    frame,
                    dynamic: self.background.dynamic(),
                    normalized: None,
                    direct_position: 0.0,
                    direct_energy: 0.0,
                    reference_weights: [1.0, 0.0],
                    reference_cross: Default::default(),
                    settling: !self.background.settled(),
                    held: true,
                    energy_floor: self.config.residual_motion,
                })?;
                events.push(EngineEvent::Features(features));
                self.detector.suspend();
                continue;
            }
            let fast = self.echo.process(
                &self.reference_window,
                &self.microphone_window,
                if self.config.resample_reference {
                    0.0
                } else {
                    fraction
                },
                self.background.settled().then(|| {
                    if self.config.residual_motion {
                        self.background.gain_matched_transfer()
                    } else {
                        self.background.transfer()
                    }
                }),
                self.running_frames >= self.format.sample_rate as u64,
            )?;
            if !fast.impulse && !self.config.residual_motion {
                self.aligner.observe_fast(fast.direct, fast.mode_changed);
            }
            if fast.mode_changed {
                let short = self.background.settled();
                self.reset_motion(short, events);
            }
            let held = fast.impulse || frame < self.model_jump_until;
            let background = if self.config.residual_motion {
                self.background.process_residual(
                    &self.echo,
                    self.echo.weights(),
                    self.motion.present(),
                    held,
                )?
            } else {
                self.background.process(
                    self.echo.transfer(),
                    self.echo.maps(),
                    self.echo.weights(),
                    self.motion.present(),
                    held,
                )?
            };
            if self.config.residual_motion && !held && !background.model_jump {
                self.observe_residual_direct(fast.mode_changed);
            }
            if background.model_jump {
                events.push(EngineEvent::Warning(Warning::ModelJump));
                self.echo.reset_averages();
                self.suspended_until = frame.saturating_add(
                    (self.config.jump_suspend_ms * self.format.sample_rate as f32 / 1000.0) as u64,
                );
                self.model_jump_until = self.suspended_until;
                self.jump_streak += 1;
                if self.config.residual_motion && self.jump_streak >= 3 {
                    // A background that no longer matches the room would otherwise hold forever.
                    self.reset_motion(true, events);
                    self.jump_streak = 0;
                }
            } else if !held {
                self.jump_streak = 0;
            }
            let length =
                (self.config.pre_taps + self.config.taps) * self.format.ref_channels as usize;
            for (own, maps) in self
                .own_directs
                .iter_mut()
                .zip(self.background.maps().chunks(length))
            {
                // Each microphone's strongest background tap is its own direct sound.
                *own = strongest_tap(maps, self.config.pre_taps + self.config.taps)
                    .map(|index| index - self.config.pre_taps as f64);
            }
            self.motion.set_own_directs(&self.own_directs);
            let origin = nearest_direct(
                self.background.maps(),
                self.config.pre_taps,
                self.config.taps,
                self.config.align_peak_ratio,
            )
            .unwrap_or(background.direct.position);
            let features = self.motion.process(MotionInput {
                frame,
                dynamic: self.background.dynamic(),
                normalized: Some(self.background.normalized_maps()),
                direct_position: origin,
                direct_energy: background.direct_energy,
                // Residual motion uses one matched filter against the mean reference.
                reference_weights: if self.config.residual_motion {
                    [1.0, 0.0]
                } else {
                    fast.reference_weights
                },
                reference_cross: if self.config.residual_motion {
                    Default::default()
                } else {
                    fast.reference_cross
                },
                settling: background.settling,
                held: fast.impulse || background.model_jump || frame < self.model_jump_until,
                energy_floor: self.config.residual_motion,
            })?;
            if features.changed {
                events.push(EngineEvent::Hand {
                    present: features.present,
                    distance_cm: features.distance_cm,
                });
            }
            events.push(EngineEvent::Features(features));
            if let Some(poke) = self.detector.process(
                features,
                !background.settling
                    && !self.paused
                    && frame >= self.suspended_until
                    && self.meter.status().permits_detection(self.config.pc_mode)
                    && self.meter.reading().score >= 15
                    && self.meter.reference_active(),
            ) {
                events.push(EngineEvent::Poke(poke));
            }
            let mut row = self.row.get();
            for (maximum, value) in row.iter_mut().zip(self.motion.take_row()) {
                *maximum = maximum.max(value);
            }
            self.row.set(row);
            self.set_state(
                if background.settling {
                    EngineState::Settling
                } else {
                    EngineState::Running
                },
                events,
            );
            if !background.settling {
                self.running_frames += self.config.hop as u64;
            }
            let impulsive = fast.impulse_fraction > 0.3;
            if impulsive && !self.impulsive_noise {
                events.push(EngineEvent::Warning(Warning::ImpulsiveNoise));
            }
            self.impulsive_noise = impulsive;
            // Residual clock feedback starts while the background is still being learned; its
            // first estimates are not a measured drift.
            let drift = self.aligner.drift_warning()
                && !(self.config.residual_motion && background.settling);
            if drift && !self.drift_warning {
                events.push(EngineEvent::Warning(Warning::Drift));
            }
            self.drift_warning = drift;
            self.latest_echo = Some(fast);
            self.meter.observe_echo(frame, fast);
        }
        Ok(())
    }
}
