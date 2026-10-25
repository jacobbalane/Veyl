use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EngineConfig {
    pub pc_mode: bool,
    pub window: usize,
    pub hop: usize,
    pub pre_taps: usize,
    pub taps: usize,
    pub f_lo: f32,
    pub f_hi: f32,
    pub edge_hz: f32,
    pub fast_seconds: f32,
    pub coherence_seconds: f32,
    pub regularization: f32,
    pub infer_single_path: bool,
    pub resample_reference: bool,
    pub residual_motion: bool,
    pub motion_seconds: f32,
    pub reference_power_seconds: f32,
    pub residual_settle_seconds: f32,
    /// Residual mode: microphone samples per change measurement, centred in the window. Zero
    /// measures change over the whole window.
    pub change_window: usize,
    /// Short change measurements: half-width in bins of the segment's own power used to
    /// whiten change. Zero uses the long-term reference spectrum.
    pub change_smooth_bins: usize,
    /// Short change measurements: residual samples whose envelope, relative to the predicted
    /// sound's, exceeds this multiple of the segment's median are a click and are zeroed. Zero
    /// keeps every sample.
    pub change_blank_ratio: f32,
    /// Short change measurements: fit the field's gain, phase and timing over the segment
    /// itself instead of reusing the whole window's fit.
    pub change_own_fit: bool,
    /// Short change measurements: form the residual within the analysis band only.
    pub change_in_band: bool,
    /// Short change measurements: a segment whose residual, relative to its prediction, exceeds
    /// this multiple of its two-second median holds motion for that hop. Zero disables it.
    pub change_impulse_ratio: f32,
    /// Short change measurements: windows whose missing frames form no gap longer than this
    /// and cover at most a quarter of the window are measured with those frames blanked, without
    /// learning. Zero leaves every incomplete window unprocessed.
    pub change_gap_ms: f32,
    pub activity_db: f32,
    pub activity_bins: usize,
    pub mono_to_stereo: f32,
    pub stereo_to_mono: f32,
    pub impulse_ratio: f32,
    /// Compare the residual relative to the predicted power with its median, rather than the
    /// residual alone.
    pub impulse_relative: bool,
    pub impulse_hold_hops: usize,
    pub settle_seconds: f32,
    pub reset_settle_seconds: f32,
    pub direct_before: usize,
    pub direct_after: usize,
    pub min_gain: f32,
    pub max_gain: f32,
    pub max_phase_step_degrees: f32,
    pub jump_suspend_ms: f32,
    pub background_seconds: f32,
    pub background_hold_seconds: f32,
    pub presence_hold_seconds: f32,
    pub floor_seconds: f32,
    pub floor_calibration_seconds: f32,
    pub floor_relative: f32,
    /// The floor learns only from hops below this fraction of the on threshold, so a weak
    /// echo of a slowly moving hand does not raise it.
    pub floor_learn_fraction: f32,
    pub z_on: f32,
    pub peakiness: f32,
    pub presence_on_hops: usize,
    pub presence_off_hops: usize,
    pub distance_smooth_seconds: f32,
    /// While a hand is present, its echo is sought within this distance of the track. Zero
    /// searches the whole range every hop.
    pub track_gate_cm: f32,
    /// A peak outside the gate this many times stronger for three hops takes the track.
    pub track_escape_ratio: f32,
    pub range_min_cm: f32,
    pub range_max_cm: f32,
    pub temperature_c: f32,
    pub poke_in_cm: f32,
    pub poke_out_cm: f32,
    pub poke_v_min: f32,
    pub poke_v_max: f32,
    pub poke_min_ms: f32,
    pub poke_max_ms: f32,
    pub poke_refractory_ms: f32,
    pub poke_valid_fraction: f32,
    /// Within 300 ms of a gap, model jump or re-lock, defer confirmation and keep the
    /// trajectory, counting unusable hops as invalid, instead of clearing it.
    pub poke_defer_suspension: bool,
    pub poke_confidence_cm: f32,
    pub ring_seconds: usize,
    pub align_seconds: f32,
    pub align_ramp_ms: f32,
    pub align_min_psr: f32,
    pub align_peak_ratio: f32,
    pub align_min_dbfs: f32,
    pub align_max_missing_fraction: f32,
    pub align_max_gap_ms: f32,
    pub align_tolerance_frames: f64,
    pub align_interval_seconds: f32,
    pub recheck_seconds: f32,
    pub drift_warning_ppm: f64,
    pub drift_phase_noise: f64,
    pub drift_rate_noise: f64,
}
impl Default for EngineConfig {
    /// The tuned engine (decisions 0020–0023).
    fn default() -> Self {
        Self {
            infer_single_path: true,
            resample_reference: true,
            residual_motion: true,
            floor_relative: 1e-8,
            impulse_ratio: 12.0,
            impulse_relative: true,
            change_window: 1024,
            change_smooth_bins: 8,
            change_in_band: true,
            change_own_fit: true,
            change_blank_ratio: 4.0,
            change_impulse_ratio: 12.0,
            change_gap_ms: 12.0,
            track_gate_cm: 4.0,
            poke_defer_suspension: true,
            floor_learn_fraction: 0.5,
            ..Self::specified()
        }
    }
}
impl EngineConfig {
    /// The values specified in plan §6, kept for component tests and comparisons.
    pub fn specified() -> Self {
        Self {
            pc_mode: false,
            window: 2048,
            hop: 256,
            pre_taps: 32,
            taps: 192,
            f_lo: 8000.0,
            f_hi: 20000.0,
            edge_hz: 500.0,
            fast_seconds: 0.04,
            coherence_seconds: 0.04,
            regularization: 1e-3,
            infer_single_path: false,
            resample_reference: false,
            residual_motion: false,
            motion_seconds: 0.005,
            reference_power_seconds: 0.5,
            residual_settle_seconds: 0.05,
            change_window: 0,
            change_smooth_bins: 0,
            change_blank_ratio: 0.0,
            change_own_fit: false,
            change_in_band: false,
            change_impulse_ratio: 0.0,
            change_gap_ms: 0.0,
            activity_db: 30.0,
            activity_bins: 9,
            mono_to_stereo: 0.95,
            stereo_to_mono: 0.98,
            impulse_ratio: 6.0,
            impulse_relative: false,
            impulse_hold_hops: 2,
            settle_seconds: 1.5,
            reset_settle_seconds: 0.5,
            direct_before: 8,
            direct_after: 6,
            min_gain: 0.25,
            max_gain: 4.0,
            max_phase_step_degrees: 90.0,
            jump_suspend_ms: 300.0,
            background_seconds: 2.0,
            background_hold_seconds: 20.0,
            presence_hold_seconds: 3.0,
            floor_seconds: 3.0,
            floor_calibration_seconds: 0.15,
            floor_relative: 1e-6,
            floor_learn_fraction: 1.0,
            z_on: 8.0,
            peakiness: 3.0,
            presence_on_hops: 3,
            presence_off_hops: 6,
            distance_smooth_seconds: 0.015,
            track_gate_cm: 0.0,
            track_escape_ratio: 2.0,
            range_min_cm: 4.0,
            range_max_cm: 40.0,
            temperature_c: 20.0,
            poke_in_cm: 3.0,
            poke_out_cm: 2.0,
            poke_v_min: 10.0,
            poke_v_max: 500.0,
            poke_min_ms: 100.0,
            poke_max_ms: 900.0,
            poke_refractory_ms: 350.0,
            poke_valid_fraction: 0.8,
            poke_defer_suspension: false,
            poke_confidence_cm: 6.0,
            ring_seconds: 4,
            align_seconds: 1.0,
            align_ramp_ms: 10.0,
            align_min_psr: 6.0,
            align_peak_ratio: 0.5,
            align_min_dbfs: -70.0,
            align_max_missing_fraction: 0.02,
            align_max_gap_ms: 10.0,
            align_tolerance_frames: 2.0,
            align_interval_seconds: 1.0,
            recheck_seconds: 5.0,
            drift_warning_ppm: 500.0,
            drift_phase_noise: 1e-5,
            drift_rate_noise: 1e-16,
        }
    }
    /// The transform size for a short change measurement: the segment plus the reference it
    /// reads, the taps' history before it and the pre-taps after it.
    pub fn change_size(&self) -> usize {
        (self.change_window + self.pre_taps + self.taps).next_power_of_two()
    }
    /// Where the short change measurement reads: the transform's first window index, and the
    /// segment's first index within the transform. The segment stays centred in the window.
    pub fn change_placement(&self) -> (usize, usize) {
        let first = self.window.saturating_sub(self.change_window) / 2;
        let origin = first
            .saturating_sub(self.taps)
            .min(self.window.saturating_sub(self.change_size()));
        (origin, first - origin.min(first))
    }
    pub fn from_toml(text: &str) -> Result<Self, &'static str> {
        if text.len() > 65536 {
            return Err("Engine configuration is too large.");
        }
        let config: Self = toml::from_str(text).map_err(|_| "Invalid engine configuration.")?;
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        let positive = [
            self.f_lo,
            self.f_hi,
            self.edge_hz,
            self.fast_seconds,
            self.coherence_seconds,
            self.motion_seconds,
            self.reference_power_seconds,
            self.residual_settle_seconds,
            self.regularization,
            self.activity_db,
            self.impulse_ratio,
            self.settle_seconds,
            self.reset_settle_seconds,
            self.min_gain,
            self.max_gain,
            self.max_phase_step_degrees,
            self.jump_suspend_ms,
            self.background_seconds,
            self.background_hold_seconds,
            self.presence_hold_seconds,
            self.floor_seconds,
            self.floor_calibration_seconds,
            self.floor_relative,
            self.z_on,
            self.peakiness,
            self.distance_smooth_seconds,
            self.range_min_cm,
            self.range_max_cm,
            self.poke_in_cm,
            self.poke_out_cm,
            self.poke_v_min,
            self.poke_v_max,
            self.poke_min_ms,
            self.poke_max_ms,
            self.poke_refractory_ms,
            self.poke_confidence_cm,
            self.align_seconds,
            self.align_ramp_ms,
            self.align_min_psr,
            self.align_interval_seconds,
            self.recheck_seconds,
        ];
        if !self.window.is_power_of_two()
            || !(512..=8192).contains(&self.window)
            || self.hop == 0
            || self.hop > self.window / 2
            || self
                .pre_taps
                .checked_add(self.taps)
                .is_none_or(|taps| taps > self.window / 2)
            || self.pre_taps == 0
            || self.taps == 0
            || (self.change_window != 0
                && (!self.change_window.is_multiple_of(32)
                    || self.change_window < 128
                    || self.change_size() > self.window
                    || self.change_placement().1 < self.taps
                    || self.change_placement().1 + self.change_window + self.pre_taps
                        > self.change_size()))
            || !(4..=16).contains(&self.ring_seconds)
            || positive
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0)
            || !self.align_peak_ratio.is_finite()
            || !(0.1..=1.0).contains(&self.align_peak_ratio)
            || self.f_lo < 6000.0
            || self.f_hi > 21000.0
            || self.f_lo >= self.f_hi
            || self.edge_hz > self.f_hi - self.f_lo
            || self.fast_seconds > 2.0
            || self.coherence_seconds > 2.0
            || self.motion_seconds > 2.0
            || self.reference_power_seconds > 30.0
            || self.residual_settle_seconds > 5.0
            || self.regularization > 1.0
            || self.activity_db > 120.0
            || self.activity_bins == 0
            || self.activity_bins.is_multiple_of(2)
            || self.activity_bins > 65
            || !self.mono_to_stereo.is_finite()
            || !self.stereo_to_mono.is_finite()
            || !(0.0..1.0).contains(&self.mono_to_stereo)
            || !(self.mono_to_stereo..=1.0).contains(&self.stereo_to_mono)
            || self.stereo_to_mono == self.mono_to_stereo
            || self.impulse_ratio <= 1.0
            || self.impulse_hold_hops > 32
            || self.settle_seconds > 30.0
            || self.reset_settle_seconds > 30.0
            || self.direct_before > self.pre_taps
            || self.direct_after > self.taps
            || self.min_gain >= self.max_gain
            || self.max_gain > 100.0
            || self.max_phase_step_degrees > 180.0
            || self.jump_suspend_ms > 10000.0
            || self.background_seconds > 300.0
            || self.background_hold_seconds < self.background_seconds
            || self.background_hold_seconds > 600.0
            || self.presence_hold_seconds > 60.0
            || self.floor_seconds > 300.0
            || self.floor_calibration_seconds > 5.0
            || self.floor_relative > 1.0
            || !self.floor_learn_fraction.is_finite()
            || !(0.05..=1.0).contains(&self.floor_learn_fraction)
            || self.z_on > 1000.0
            || self.peakiness > 1000.0
            || !(1..=1000).contains(&self.presence_on_hops)
            || !(1..=1000).contains(&self.presence_off_hops)
            || self.distance_smooth_seconds > 1.0
            || !self.change_gap_ms.is_finite()
            || !(0.0..=50.0).contains(&self.change_gap_ms)
            || !self.change_impulse_ratio.is_finite()
            || !(0.0..=1000.0).contains(&self.change_impulse_ratio)
            || (self.change_impulse_ratio > 0.0 && self.change_impulse_ratio <= 1.0)
            || !self.change_blank_ratio.is_finite()
            || !(0.0..=1000.0).contains(&self.change_blank_ratio)
            || !self.track_escape_ratio.is_finite()
            || !(1.0..=100.0).contains(&self.track_escape_ratio)
            || !self.track_gate_cm.is_finite()
            || !(0.0..=60.0).contains(&self.track_gate_cm)
            || self.range_min_cm >= self.range_max_cm
            || self.range_max_cm > 60.0
            || !self.temperature_c.is_finite()
            || !(-40.0..=60.0).contains(&self.temperature_c)
            || self.poke_in_cm > 60.0
            || self.poke_out_cm > 60.0
            || self.poke_v_min > 1000.0
            || self.poke_v_max < self.poke_v_min
            || self.poke_v_max > 10000.0
            || self.poke_min_ms > self.poke_max_ms
            || self.poke_max_ms > 5000.0
            || self.poke_refractory_ms > 10000.0
            || !self.poke_valid_fraction.is_finite()
            || !(0.5..=1.0).contains(&self.poke_valid_fraction)
            || self.poke_confidence_cm > 60.0
            || self.align_seconds > 1.0
            || self.align_seconds < 0.1
            || self.align_ramp_ms >= self.align_seconds * 500.0
            || !self.align_min_dbfs.is_finite()
            || !(-120.0..=0.0).contains(&self.align_min_dbfs)
            || !self.align_max_missing_fraction.is_finite()
            || !(0.0..=0.1).contains(&self.align_max_missing_fraction)
            || !self.align_max_gap_ms.is_finite()
            || !(0.0..=50.0).contains(&self.align_max_gap_ms)
            || !self.align_tolerance_frames.is_finite()
            || !(0.1..=16.0).contains(&self.align_tolerance_frames)
            || self.align_interval_seconds > 60.0
            || self.recheck_seconds < self.align_interval_seconds
            || self.recheck_seconds > 60.0
            || [
                self.drift_warning_ppm,
                self.drift_phase_noise,
                self.drift_rate_noise,
            ]
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
            || self.drift_phase_noise > 16.0
            || self.drift_rate_noise > 1e-6
            || self.drift_warning_ppm > 100000.0
        {
            return Err("Invalid engine parameters.");
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toml_defaults_and_overrides_reject_typos_nonfinite_and_unsafe_dimensions() {
        let config = EngineConfig::from_toml("align_min_psr = 7.0\n").unwrap();
        assert_eq!(config.window, 2048);
        assert_eq!(config.align_min_psr, 7.0);
        assert!(EngineConfig::from_toml("alig_min_psr = 7.0").is_err());
        assert!(EngineConfig::from_toml("f_lo = nan").is_err());
        assert!(EngineConfig::from_toml("hop = 0").is_err());
        for text in [
            "align_max_missing_fraction = -0.01",
            "align_max_missing_fraction = 0.11",
            "align_max_missing_fraction = nan",
            "align_max_gap_ms = -1.0",
            "align_max_gap_ms = 51.0",
            "align_max_gap_ms = inf",
        ] {
            assert!(EngineConfig::from_toml(text).is_err());
        }
        assert!(EngineConfig::from_toml(
            "align_max_missing_fraction = 0.0\nalign_max_gap_ms = 0.0"
        )
        .is_ok());
        // A short change segment needs room for every tap inside the window.
        for text in [
            "change_window = 96",
            "change_window = 1000",
            "change_window = 2048",
            "track_gate_cm = -1.0",
            "track_gate_cm = nan",
        ] {
            assert!(EngineConfig::from_toml(text).is_err(), "{text}");
        }
        let short = EngineConfig::from_toml("change_window = 1024\ntrack_gate_cm = 4.0").unwrap();
        assert_eq!(short.change_size(), 2048);
        assert_eq!(short.change_placement(), (0, 512));
        let shorter = EngineConfig::from_toml("change_window = 768").unwrap();
        assert_eq!(shorter.change_size(), 1024);
        assert_eq!(shorter.change_placement(), (448, 192));
        let text = toml::to_string(&config).unwrap();
        assert_eq!(
            EngineConfig::from_toml(&text).unwrap().drift_rate_noise,
            config.drift_rate_noise
        );
    }
}
