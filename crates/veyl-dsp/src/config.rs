use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EngineConfig {
    pub window: usize,
    pub hop: usize,
    pub pre_taps: usize,
    pub taps: usize,
    pub f_lo: f32,
    pub f_hi: f32,
    pub edge_hz: f32,
    pub fast_seconds: f32,
    pub regularization: f32,
    pub activity_db: f32,
    pub activity_bins: usize,
    pub mono_to_stereo: f32,
    pub stereo_to_mono: f32,
    pub impulse_ratio: f32,
    pub impulse_hold_hops: usize,
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
            window: 2048,
            hop: 256,
            pre_taps: 32,
            taps: 192,
            f_lo: 8000.0,
            f_hi: 20000.0,
            edge_hz: 500.0,
            fast_seconds: 0.04,
            regularization: 1e-3,
            activity_db: 30.0,
            activity_bins: 9,
            mono_to_stereo: 0.95,
            stereo_to_mono: 0.98,
            impulse_ratio: 6.0,
            impulse_hold_hops: 2,
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
            self.regularization,
            self.activity_db,
            self.impulse_ratio,
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
