//! Stream statistics are available before M4 adds echo and gesture analysis.
use crate::recording::Input;
use serde::Serialize;
use std::{fs, io, path::Path};

#[derive(Clone, Default, Serialize)]
pub struct StreamStatistics {
    pub blocks: u64,
    pub received_frames: u64,
    pub span_frames: u64,
    pub missing_frames: u64,
    pub sequence_gaps: u64,
    pub overlaps: u64,
    pub flagged_gaps: u64,
    pub unknown_timestamps: u64,
    pub mean_pre_rms_dbfs: Option<f64>,
    pub filtered_rms_dbfs: Option<f64>,
    pub pre_peak: f32,
}
#[derive(Clone, Serialize)]
pub struct Summary {
    pub format: u32,
    pub analysis: &'static str,
    pub sample_rate: u32,
    pub source: String,
    pub hp_cutoff_hz: u32,
    pub reference: StreamStatistics,
    pub microphone: StreamStatistics,
    pub labels: usize,
}
pub fn statistics(input: &Input) -> io::Result<Summary> {
    let mut streams = [StreamStatistics::default(), StreamStatistics::default()];
    let mut expected_frame = [None, None];
    let mut expected_seq: [Option<u32>; 2] = [None, None];
    let mut power = [0.0, 0.0];
    let mut pre_sum = [0.0, 0.0];
    let mut samples = [0_u64, 0];
    for index in 0..input.blocks.len() {
        let block = input.block(index, 0)?;
        let stream = block.stream as usize;
        let stats = &mut streams[stream];
        let end = block
            .first_frame
            .checked_add(block.frames as u64)
            .ok_or_else(|| io::Error::other("Recording frame overflow."))?;
        let first = if stream == 0 {
            input.meta.ref_first_frame
        } else {
            input.meta.mic_first_frame
        };
        let expected = expected_frame[stream].unwrap_or(first);
        if block.first_frame > expected {
            stats.missing_frames += block.first_frame - expected;
        }
        if block.first_frame < expected {
            stats.overlaps += 1;
        }
        if let Some(expected) = expected_seq[stream] {
            let missing = block.seq.wrapping_sub(expected);
            if missing < u32::MAX / 2 {
                stats.sequence_gaps += missing as u64;
            }
        }
        expected_seq[stream] = Some(block.seq.wrapping_add(1));
        expected_frame[stream] = Some(expected.max(end));
        stats.blocks += 1;
        stats.received_frames += block.frames as u64;
        stats.span_frames = stats.span_frames.max(end - first);
        stats.flagged_gaps += u64::from(block.flags & 1 != 0);
        stats.unknown_timestamps += u64::from(block.capture_ns == i64::MIN);
        stats.pre_peak = stats.pre_peak.max(block.pre_peak);
        pre_sum[stream] += block.pre_rms_dbfs as f64;
        power[stream] += block
            .samples
            .iter()
            .map(|&value| (value as f64 / 32768.0).powi(2))
            .sum::<f64>();
        samples[stream] += block.samples.len() as u64;
    }
    for index in 0..2 {
        if streams[index].blocks > 0 {
            streams[index].mean_pre_rms_dbfs = Some(pre_sum[index] / streams[index].blocks as f64);
            streams[index].filtered_rms_dbfs =
                Some(10.0 * (power[index] / samples[index] as f64).max(1e-12).log10());
        }
    }
    let [reference, microphone] = streams;
    Ok(Summary {
        format: 1,
        analysis: "stream_statistics",
        sample_rate: input.meta.sample_rate,
        source: input.meta.source.clone(),
        hp_cutoff_hz: input.meta.hp_cutoff_hz,
        reference,
        microphone,
        labels: input.labels.len(),
    })
}
pub fn write_summary(input: &Input, output: &Path) -> io::Result<Summary> {
    let summary = statistics(input)?;
    fs::create_dir_all(output)?;
    let bytes = serde_json::to_vec_pretty(&summary).map_err(io::Error::other)?;
    fs::write(output.join("summary.json"), bytes)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn statistics_distinguish_missing_frames_flags_unknown_clocks_and_overlap() {
        let mut input = Input::generated(veyl_sim::poke_basic(1, 0.05).unwrap());
        input.blocks.retain(|block| block.seq != 2);
        input.blocks[0].capture_ns = i64::MIN;
        input.blocks[4].flags |= 1;
        let duplicate = input.blocks.last().unwrap().clone();
        input.blocks.push(duplicate);
        let summary = statistics(&input).unwrap();
        assert_eq!(summary.reference.blocks, 4);
        assert_eq!(summary.reference.missing_frames, 480);
        assert_eq!(summary.reference.sequence_gaps, 1);
        assert_eq!(summary.reference.flagged_gaps, 1);
        assert_eq!(summary.reference.unknown_timestamps, 1);
        assert_eq!(summary.microphone.overlaps, 1);
        assert!(
            summary.reference.filtered_rms_dbfs.unwrap()
                < summary.reference.mean_pre_rms_dbfs.unwrap()
        );
        let output = tempfile::tempdir().unwrap();
        write_summary(&input, output.path()).unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(output.path().join("summary.json")).unwrap()).unwrap();
        assert_eq!(value["analysis"], "stream_statistics");
        assert!(value.get("pokes").is_none());
    }
}
