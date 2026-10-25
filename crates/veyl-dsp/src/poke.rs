//! Confirm a fast approach, turn and retreat using frame-indexed, bounded history.
use crate::{config::EngineConfig, motion::MotionFeatures, RangePreset, Sensitivity};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PokeEvent {
    pub turn_frame: u64,
    pub detect_frame: u64,
    pub distance_cm: f32,
    pub depth_cm: f32,
    pub confidence: f32,
}
#[derive(Clone, Copy)]
struct Entry {
    frame: u64,
    distance: Option<f32>,
    z: f32,
}
pub struct PokeDetector {
    config: EngineConfig,
    rate: u32,
    range: (f32, f32),
    z_on: f32,
    thresholds: (f32, f32, f32),
    history: VecDeque<Entry>,
    minimum: u64,
    maximum: u64,
    refractory: u64,
    refractory_until: u64,
}
impl PokeDetector {
    pub fn new(config: EngineConfig, rate: u32) -> Result<Self, &'static str> {
        config.validate()?;
        if ![44100, 48000].contains(&rate) {
            return Err("Unsupported poke sample rate.");
        }
        let frames = |ms: f32| (ms * rate as f32 / 1000.0).round() as u64;
        let maximum = frames(config.poke_max_ms);
        Ok(Self {
            rate,
            range: (config.range_min_cm, config.range_max_cm),
            z_on: config.z_on,
            thresholds: (config.poke_in_cm, config.poke_out_cm, config.poke_v_min),
            history: VecDeque::with_capacity((maximum as usize).div_ceil(config.hop) + 2),
            minimum: frames(config.poke_min_ms),
            maximum,
            refractory: frames(config.poke_refractory_ms),
            refractory_until: 0,
            config,
        })
    }
    pub fn suspend(&mut self) {
        self.history.clear();
    }
    pub fn reset(&mut self) {
        self.suspend();
        self.refractory_until = 0;
    }
    pub fn set_sensitivity(&mut self, preset: Sensitivity) {
        self.z_on = preset.z_on();
        self.thresholds = preset.poke_thresholds();
        self.suspend();
    }
    pub fn set_range(&mut self, preset: RangePreset) {
        self.range = preset.limits_cm();
        self.suspend();
    }
    pub fn process(&mut self, feature: MotionFeatures, allowed: bool) -> Option<PokeEvent> {
        let frame = feature.frame;
        if !allowed || !feature.present || frame < self.refractory_until {
            self.suspend();
            return None;
        }
        if self
            .history
            .back()
            .is_some_and(|entry| entry.frame >= frame)
        {
            return None;
        }
        let oldest = frame.saturating_sub(self.maximum);
        while self
            .history
            .front()
            .is_some_and(|entry| entry.frame < oldest)
        {
            self.history.pop_front();
        }
        if self.history.len() == self.history.capacity() {
            self.history.pop_front();
        }
        let distance = feature
            .distance_cm
            .filter(|value| feature.valid && value.is_finite());
        if let Some(current) = distance {
            if let Some(previous) = self
                .history
                .iter()
                .rev()
                .find(|entry| entry.distance.is_some())
            {
                let speed = (current - previous.distance.unwrap()).abs() * self.rate as f32
                    / (frame - previous.frame) as f32;
                if speed > self.config.poke_v_max {
                    // A jump to another echo cannot form part of a hand trajectory.
                    self.suspend();
                }
            }
        }
        self.history.push_back(Entry {
            frame,
            distance,
            z: feature.z_max,
        });
        let current = distance?;
        // The final equal minimum is the turn after a brief stationary hold.
        let mut turn = None;
        for entry in &self.history {
            if let Some(value) = entry.distance {
                if turn.is_none_or(|old: Entry| value <= old.distance.unwrap()) {
                    turn = Some(*entry);
                }
            }
        }
        let turn = turn?;
        let closest = turn.distance?;
        // The approach begins where the hand closed in deeply and quickly enough. The farthest
        // point alone would include a slow drift into position and dilute the approach speed.
        let mut approach = None;
        for entry in self
            .history
            .iter()
            .take_while(|entry| entry.frame < turn.frame)
        {
            let Some(value) = entry.distance else {
                continue;
            };
            let depth = value - closest;
            let in_frames = turn.frame - entry.frame;
            if depth >= self.thresholds.0
                && depth * self.rate as f32 / in_frames as f32 >= self.thresholds.2
                && (self.minimum..=self.maximum).contains(&(frame - entry.frame))
                && approach.is_none_or(|(_, deepest)| depth > deepest)
            {
                approach = Some((*entry, depth));
            }
        }
        let (approach, depth) = approach?;
        let rise = current - closest;
        let duration = frame - approach.frame;
        let out_frames = frame - turn.frame;
        if rise < self.thresholds.1
            || turn.z < self.z_on
            || out_frames == 0
            || rise * self.rate as f32 / (out_frames as f32) < self.thresholds.2
            || !(self.range.0..=self.range.1).contains(&closest)
        {
            return None;
        }
        let valid = self
            .history
            .iter()
            .filter(|entry| entry.frame >= approach.frame && entry.distance.is_some())
            .count();
        let expected = duration / self.config.hop as u64 + 1;
        if valid as f32 / (expected as f32) < self.config.poke_valid_fraction {
            return None;
        }
        let event = PokeEvent {
            turn_frame: turn.frame,
            detect_frame: frame,
            distance_cm: closest,
            depth_cm: depth,
            confidence: (depth / self.config.poke_confidence_cm).clamp(0.0, 1.0)
                * (turn.z / (2.0 * self.z_on)).clamp(0.0, 1.0),
        };
        self.refractory_until = frame.saturating_add(self.refractory);
        self.suspend();
        Some(event)
    }
}

#[cfg(test)]
mod tests;
