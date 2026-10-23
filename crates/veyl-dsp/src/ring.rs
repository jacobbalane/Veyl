//! Absolute frame tags make missing, overwritten and overlapping input explicit.
use std::collections::VecDeque;
#[derive(Clone, Copy)]
struct Anchor {
    frame: u64,
    ns: i64,
}
pub struct SampleRing {
    channels: usize,
    capacity: usize,
    rate: u32,
    samples: Vec<f32>,
    tags: Vec<u64>,
    end: Option<u64>,
    first: Option<u64>,
    anchors: VecDeque<Anchor>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct PushResult {
    pub frames: usize,
    pub gap: bool,
    pub overlap: usize,
}
impl SampleRing {
    pub fn new(rate: u32, channels: u8, seconds: usize) -> Result<Self, &'static str> {
        if ![44100, 48000].contains(&rate)
            || !(1..=4).contains(&channels)
            || !(4..=16).contains(&seconds)
        {
            return Err("Invalid sample ring dimensions.");
        }
        let capacity = rate as usize * seconds;
        Ok(Self {
            channels: channels as usize,
            capacity,
            rate,
            samples: vec![0.0; capacity * channels as usize],
            tags: vec![u64::MAX; capacity],
            end: None,
            first: None,
            anchors: VecDeque::with_capacity(seconds * 128),
        })
    }
    pub fn end(&self) -> Option<u64> {
        self.end
    }
    pub fn first(&self) -> Option<u64> {
        self.first
    }
    pub fn oldest(&self) -> u64 {
        self.end.unwrap_or(0).saturating_sub(self.capacity as u64)
    }
    pub fn clear(&mut self) {
        self.tags.fill(u64::MAX);
        self.end = None;
        self.first = None;
        self.anchors.clear();
    }
    pub fn push(
        &mut self,
        first: u64,
        samples: &[i16],
        capture_ns: Option<i64>,
        gap_before: bool,
    ) -> Result<PushResult, &'static str> {
        if !samples.len().is_multiple_of(self.channels) {
            return Err("Partial interleaved frame.");
        }
        let frames = samples.len() / self.channels;
        let last = first
            .checked_add(frames as u64)
            .ok_or("Audio frame overflow.")?;
        if frames == 0 {
            return Ok(PushResult::default());
        }
        self.first.get_or_insert(first);
        let overlap = self
            .end
            .unwrap_or(first)
            .saturating_sub(first)
            .min(frames as u64) as usize;
        if overlap == frames {
            return Ok(PushResult {
                overlap,
                ..Default::default()
            });
        }
        let gap = self.end.is_some_and(|end| first > end) || gap_before;
        let start = overlap.max(frames.saturating_sub(self.capacity));
        for index in start..frames {
            let frame = first + index as u64;
            let slot = (frame % self.capacity as u64) as usize;
            self.tags[slot] = frame;
            for channel in 0..self.channels {
                self.samples[slot * self.channels + channel] =
                    samples[index * self.channels + channel] as f32 / 32768.0;
            }
        }
        // A flagged discontinuity without a numbered gap still invalidates its boundary.
        if gap_before && self.end == Some(first) {
            self.tags[(first % self.capacity as u64) as usize] = u64::MAX;
        }
        self.end = Some(last);
        if let Some(ns) = capture_ns.filter(|ns| *ns != i64::MIN) {
            if self.anchors.len() == self.anchors.capacity() {
                self.anchors.pop_front();
            }
            self.anchors.push_back(Anchor { frame: first, ns });
        }
        while self
            .anchors
            .front()
            .is_some_and(|anchor| anchor.frame < self.oldest())
        {
            self.anchors.pop_front();
        }
        Ok(PushResult {
            frames: frames - overlap,
            gap,
            overlap,
        })
    }
    pub fn copy(&self, first: u64, channel: Option<usize>, output: &mut [f32]) -> bool {
        let mut valid = channel.is_none_or(|channel| channel < self.channels);
        let end = first.checked_add(output.len() as u64);
        valid &= first >= self.oldest()
            && end.is_some_and(|end| self.end.is_some_and(|latest| end <= latest));
        for (index, sample) in output.iter_mut().enumerate() {
            let Some(frame) = first.checked_add(index as u64) else {
                *sample = 0.0;
                valid = false;
                continue;
            };
            let slot = (frame % self.capacity as u64) as usize;
            if self.tags[slot] != frame
                || frame < self.oldest()
                || self.end.is_none_or(|end| frame >= end)
                || channel.is_some_and(|channel| channel >= self.channels)
            {
                *sample = 0.0;
                valid = false;
                continue;
            }
            *sample = match channel {
                Some(channel) => self.samples[slot * self.channels + channel],
                None => {
                    self.samples[slot * self.channels..(slot + 1) * self.channels]
                        .iter()
                        .sum::<f32>()
                        / self.channels as f32
                }
            };
        }
        valid
    }
    pub fn capture_time(&self, frame: u64) -> Option<i64> {
        let anchor = self
            .anchors
            .iter()
            .min_by_key(|anchor| anchor.frame.abs_diff(frame))?;
        let delta = (frame as i128 - anchor.frame as i128) * 1_000_000_000 / self.rate as i128;
        i64::try_from(anchor.ns as i128 + delta).ok()
    }
    pub fn frame_at_time(&self, ns: i64) -> Option<f64> {
        let anchor = self
            .anchors
            .iter()
            .min_by_key(|anchor| (anchor.ns as i128 - ns as i128).abs())?;
        Some(anchor.frame as f64 + (ns as i128 - anchor.ns as i128) as f64 * self.rate as f64 / 1e9)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gaps_overlaps_large_jumps_and_wrapping_never_make_old_audio_valid() {
        let mut ring = SampleRing::new(48000, 2, 4).unwrap();
        ring.push(10, &[32767, -32768, 8192, 16384], Some(1000000), false)
            .unwrap();
        let mut mono = [0.0; 2];
        assert!(ring.copy(10, None, &mut mono));
        assert!((mono[1] - 0.375).abs() < 1e-7);
        assert_eq!(ring.capture_time(11), Some(1020833));
        assert!((ring.frame_at_time(1000000).unwrap() - 10.0).abs() < 1e-10);
        assert_eq!(
            ring.push(11, &[1, 2, 3, 4], None, false).unwrap().overlap,
            1
        );
        assert!(ring.push(15, &[5, 6], None, true).unwrap().gap);
        assert!(!ring.copy(12, None, &mut mono));
        let far = 1_000_000;
        ring.push(far, &[1, 2, 3, 4], None, false).unwrap();
        assert!(!ring.copy(10, None, &mut mono));
        assert!(ring.copy(far, Some(1), &mut mono));
        ring.push(far + 2, &[1, 2], None, true).unwrap();
        assert!(!ring.copy(far + 1, None, &mut mono));
        assert!(ring.push(u64::MAX, &[0, 0], None, false).is_err());
        ring.clear();
        assert!(ring.end().is_none());
    }
}
