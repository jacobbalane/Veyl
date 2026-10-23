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
    pub activity_db: f32,
    pub activity_bins: usize,
    pub mono_to_stereo: f32,
    pub stereo_to_mono: f32,
    pub impulse_ratio: f32,
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
    pub z_on: f32,
    pub peakiness: f32,
    pub presence_on_hops: usize,
    pub presence_off_hops: usize,
    pub distance_smooth_seconds: f32,
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
    pub poke_confidence_cm: f32,
    pub ring_seconds: usize,
    pub align_seconds: f32,
    pub align_ramp_ms: f32,
    pub align_min_psr: f32,
    pub align_peak_ratio: f32,
    pub align_min_dbfs: f32,
    pub align_tolerance_frames: f64,
    pub align_interval_seconds: f32,
    pub recheck_seconds: f32,
    pub drift_warning_ppm: f64,
    pub drift_phase_noise: f64,
    pub drift_rate_noise: f64,
}
impl Default for EngineConfig {
    fn default() -> Self {
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
            activity_db: 30.0,
            activity_bins: 9,
            mono_to_stereo: 0.95,
            stereo_to_mono: 0.98,
            impulse_ratio: 6.0,
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
            z_on: 8.0,
            peakiness: 3.0,
            presence_on_hops: 3,
            presence_off_hops: 6,
            distance_smooth_seconds: 0.015,
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
            poke_confidence_cm: 6.0,
            ring_seconds: 4,
            align_seconds: 1.0,
            align_ramp_ms: 10.0,
            align_min_psr: 6.0,
            align_peak_ratio: 0.5,
            align_min_dbfs: -70.0,
            align_tolerance_frames: 2.0,
            align_interval_seconds: 1.0,
            recheck_seconds: 5.0,
            drift_warning_ppm: 500.0,
            drift_phase_noise: 1e-5,
            drift_rate_noise: 1e-16,
        }
    }
}
impl EngineConfig {
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
            || self.z_on > 1000.0
            || self.peakiness > 1000.0
            || !(1..=1000).contains(&self.presence_on_hops)
            || !(1..=1000).contains(&self.presence_off_hops)
            || self.distance_smooth_seconds > 1.0
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
        let text = toml::to_string(&config).unwrap();
        assert_eq!(
            EngineConfig::from_toml(&text).unwrap().drift_rate_noise,
            config.drift_rate_noise
        );
    }
}
