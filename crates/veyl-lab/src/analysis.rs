//! Offline engine replay and measurements. Labels are used only after sensing has finished.
use crate::{
    analyze::{self, Summary},
    recording::Input,
};
use serde::Serialize;
use std::{collections::BTreeSet, io};
use veyl_dsp::{
    config::EngineConfig, motion::MotionFeatures, BlockFlags, BlockIn, Engine, EngineEvent,
    EngineState, MeterFactors, MeterReading, PokeEvent, SoundRoute, SoundSource, SourceStatus,
    Stream, StreamFormat, Warning,
};
use veyl_sim::scene::SceneEvent;
mod export;
pub use export::write_outputs;
#[cfg(test)]
mod tests;

#[derive(Clone, Default, Serialize)]
pub struct EngineStatistics {
    pub lock_ms: Option<f64>,
    pub lag_frames: Option<i64>,
    pub lag_ms: Option<f64>,
    pub psr: Option<f32>,
    pub relocks: u64,
    pub model_jumps: u64,
    pub drift_warnings: u64,
    pub processed_hops: u64,
    pub invalid_hops: u64,
    pub max_direct_residual_after_5s: Option<f64>,
    pub final_drift_ppm: f64,
    pub final_state: EngineState,
}
#[derive(Clone, Default, Serialize)]
pub struct MeterStatistics {
    pub readings: u64,
    pub mean_score: f64,
    pub mean_factors: MeterFactors,
    pub mean_bandwidth_hz: f64,
    pub mean_clarity_db: f64,
}
#[derive(Clone, Default, Serialize)]
pub struct PokeStatistics {
    pub detected: usize,
    pub labels: usize,
    pub matched: usize,
    pub unmatched_detections: usize,
    pub false_pokes: Option<usize>,
    pub annotations_available: bool,
    pub recall: Option<f64>,
    pub precision: Option<f64>,
    pub f1: Option<f64>,
    pub false_pokes_per_minute: Option<f64>,
    pub latency_median_ms: Option<f64>,
    pub latency_p95_ms: Option<f64>,
    pub confirmation_median_ms: Option<f64>,
    pub confirmation_p95_ms: Option<f64>,
    pub human_mark_offset_median_ms: Option<f64>,
    pub truth_labels: usize,
    pub human_marks: usize,
}
#[derive(Clone, Serialize)]
pub struct Report {
    #[serde(flatten)]
    pub streams: Summary,
    pub config: EngineConfig,
    pub engine: EngineStatistics,
    pub meter: MeterStatistics,
    pub pokes: PokeStatistics,
}
pub struct FeatureRow {
    pub feature: MotionFeatures,
    pub state: EngineState,
    pub suspended: bool,
    pub drift_ppm: f64,
    pub direct_residual: Option<f64>,
}
pub struct MotionRow {
    pub frame: u64,
    pub values: [f32; 60],
    pub valid: bool,
}
pub struct Run {
    pub report: Report,
    pub features: Vec<FeatureRow>,
    pub meters: Vec<MeterReading>,
    pub pokes: Vec<PokeEvent>,
    pub motion: Vec<MotionRow>,
    pub(crate) trace_collected: bool,
}

/// Recording mode is control metadata; gesture labels never select a sensing configuration.
pub fn default_config(input: &Input) -> EngineConfig {
    let pc = input.meta.source == "pc"
        || input.meta.device.get("mode").and_then(|v| v.as_str()) == Some("pc");
    EngineConfig {
        pc_mode: pc,
        f_lo: input.meta.hp_cutoff_hz as f32,
        range_max_cm: if pc { 55.0 } else { 40.0 },
        ..EngineConfig::default()
    }
}
fn source_status(input: &Input, pc: bool) -> SourceStatus {
    let source = match input
        .meta
        .device
        .get("sound_source")
        .and_then(|v| v.as_str())
        .unwrap_or(&input.meta.source)
    {
        "capture" => SoundSource::Capture,
        "player" => SoundSource::Player,
        "pc" => SoundSource::Pc,
        _ => {
            if pc {
                SoundSource::Pc
            } else {
                SoundSource::TestSound
            }
        }
    };
    let route = match input.meta.device.get("route").and_then(|v| v.as_str()) {
        Some("external") => SoundRoute::External,
        Some("headphones") => SoundRoute::Headphones,
        Some("bluetooth") => SoundRoute::Bluetooth,
        _ => SoundRoute::Speaker,
    };
    SourceStatus {
        capture_running: true,
        source,
        route,
        media_volume: input
            .meta
            .device
            .get("media_volume")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32),
        ..SourceStatus::default()
    }
}
pub fn evaluate(input: &Input, config: EngineConfig, trace: bool) -> io::Result<Run> {
    let stats = analyze::statistics(input)?;
    evaluate_with_statistics(input, config, stats, trace)
}
pub fn evaluate_with_statistics(
    input: &Input,
    config: EngineConfig,
    mut streams: Summary,
    trace: bool,
) -> io::Result<Run> {
    streams.analysis = "echo_engine";
    let rate = input.meta.sample_rate;
    let first = input.meta.mic_first_frame;
    let mut engine = Engine::new(
        config.clone(),
        StreamFormat {
            sample_rate: rate,
            ref_channels: input.meta.ref_channels,
            mic_channels: input.meta.mic_channels,
        },
    )
    .map_err(io::Error::other)?;
    let controls: Vec<SceneEvent> = input
        .meta
        .device
        .get("scene_events")
        .map_or(Ok(vec![]), |value| serde_json::from_value(value.clone()))
        .map_err(io::Error::other)?;
    if controls.len() > 1024
        || controls
            .iter()
            .any(|event| !event.at().is_finite() || event.at() < 0.0)
        || controls.windows(2).any(|pair| pair[0].at() > pair[1].at())
    {
        return Err(io::Error::other("Invalid source-event timeline."));
    }
    let mut status = source_status(input, config.pc_mode);
    if controls
        .iter()
        .any(|event| matches!(event, SceneEvent::CaptureBlocked { .. }))
    {
        status.source = SoundSource::Capture;
    }
    engine.set_source_status(status).map_err(io::Error::other)?;
    let base_volume = status.media_volume;
    let mut next_control = 0;
    let mut events = Vec::with_capacity(32);
    let mut report = EngineStatistics::default();
    let mut meter = MeterStatistics::default();
    let mut features = vec![];
    let mut meters = vec![];
    let mut pokes = vec![];
    let mut motion = vec![];
    let mut latest_mic_end = first;
    for index in 0..input.blocks.len() {
        let block = input.block(index, 0)?;
        let time = if block.capture_ns != i64::MIN {
            block.capture_ns.max(0) as f64 / 1e9
        } else {
            input.due_ns(index)? as f64 / 1e9
        };
        while controls
            .get(next_control)
            .is_some_and(|event| event.at() <= time)
        {
            match controls[next_control] {
                SceneEvent::Route { external, .. } => {
                    status.route = if external {
                        SoundRoute::External
                    } else {
                        SoundRoute::Speaker
                    }
                }
                SceneEvent::Volume { gain, .. } => {
                    if !gain.is_finite() || !(0.0..=4.0).contains(&gain) {
                        return Err(io::Error::other("Invalid source volume event."));
                    }
                    status.media_volume = base_volume.map(|volume| (volume * gain).clamp(0.0, 1.0));
                }
                SceneEvent::CaptureBlocked { .. } => {}
            }
            engine.set_source_status(status).map_err(io::Error::other)?;
            next_control += 1;
        }
        if block.stream == 1 {
            latest_mic_end = latest_mic_end.max(
                block
                    .first_frame
                    .checked_add(block.frames as u64)
                    .ok_or_else(|| io::Error::other("Recording frame overflow."))?,
            );
        }
        engine
            .push(BlockIn {
                stream: if block.stream == 0 {
                    Stream::Reference
                } else {
                    Stream::Microphone
                },
                first_frame: block.first_frame,
                capture_ns: (block.capture_ns != i64::MIN).then_some(block.capture_ns),
                flags: BlockFlags(block.flags),
                pre_rms_dbfs: block.pre_rms_dbfs,
                pre_peak: block.pre_peak,
                samples: &block.samples,
            })
            .map_err(io::Error::other)?;
        events.clear();
        engine.process(&mut events).map_err(io::Error::other)?;
        let snapshot = engine.snapshot();
        let mut had_features = false;
        let mut valid_features = true;
        for event in &events {
            match event {
                EngineEvent::Aligned {
                    lag_frames, psr, ..
                } => {
                    report.lock_ms.get_or_insert(
                        latest_mic_end.saturating_sub(first) as f64 * 1000.0 / rate as f64,
                    );
                    report.lag_frames = Some(*lag_frames);
                    report.psr = Some(*psr);
                }
                EngineEvent::Warning(Warning::Relocked) => report.relocks += 1,
                EngineEvent::Warning(Warning::ModelJump) => report.model_jumps += 1,
                EngineEvent::Warning(Warning::Drift) => report.drift_warnings += 1,
                EngineEvent::Poke(poke) => pokes.push(*poke),
                EngineEvent::Meter(reading) => {
                    meter.readings += 1;
                    meter.mean_score += reading.score as f64;
                    meter.mean_bandwidth_hz += reading.bandwidth_hz as f64;
                    meter.mean_clarity_db += reading.clarity_db as f64;
                    meter.mean_factors.high += reading.factors.high;
                    meter.mean_factors.detail += reading.factors.detail;
                    meter.mean_factors.volume += reading.factors.volume;
                    meter.mean_factors.steady += reading.factors.steady;
                    meter.mean_factors.clarity += reading.factors.clarity;
                    if trace {
                        meters.push(*reading);
                    }
                }
                EngineEvent::Features(feature) => {
                    if !feature.z_max.is_finite()
                        || !feature.mean_energy.is_finite()
                        || feature.distance_cm.is_some_and(|v| !v.is_finite())
                    {
                        return Err(io::Error::other(
                            "The engine produced a non-finite feature.",
                        ));
                    }
                    had_features = true;
                    valid_features &= feature.valid && !feature.settling;
                    if trace {
                        features.push(FeatureRow {
                            feature: *feature,
                            state: if feature.settling {
                                EngineState::Settling
                            } else {
                                snapshot.state
                            },
                            suspended: snapshot.suspended,
                            drift_ppm: snapshot.drift_ppm,
                            direct_residual: snapshot.direct_residual,
                        });
                    }
                }
                _ => {}
            }
        }
        if trace && had_features {
            motion.push(MotionRow {
                frame: snapshot.frame.unwrap_or(latest_mic_end),
                values: snapshot.motion,
                valid: valid_features,
            });
        }
        if let Some(residual) = snapshot.direct_residual.filter(|_| {
            snapshot
                .frame
                .is_some_and(|frame| frame.saturating_sub(first) >= 5 * rate as u64)
        }) {
            report.max_direct_residual_after_5s = Some(
                report
                    .max_direct_residual_after_5s
                    .unwrap_or(0.0)
                    .max(residual.abs()),
            );
        }
        report.lag_frames = snapshot.lag_frames.or(report.lag_frames);
        report.processed_hops = snapshot.processed_hops;
        report.invalid_hops = snapshot.invalid_hops;
        report.final_drift_ppm = snapshot.drift_ppm;
        report.final_state = snapshot.state;
    }
    report.lag_ms = report
        .lag_frames
        .map(|lag| lag as f64 * 1000.0 / rate as f64);
    let count = meter.readings.max(1) as f64;
    meter.mean_score /= count;
    meter.mean_bandwidth_hz /= count;
    meter.mean_clarity_db /= count;
    meter.mean_factors.high /= count as f32;
    meter.mean_factors.detail /= count as f32;
    meter.mean_factors.volume /= count as f32;
    meter.mean_factors.steady /= count as f32;
    meter.mean_factors.clarity /= count as f32;
    let measured = measure_pokes(input, &pokes, streams.microphone.span_frames);
    Ok(Run {
        report: Report {
            streams,
            config,
            engine: report,
            meter,
            pokes: measured,
        },
        features,
        meters,
        pokes,
        motion,
        trace_collected: trace,
    })
}
pub(crate) fn percentile(values: &mut [f64], fraction: f64) -> Option<f64> {
    values.sort_unstable_by(f64::total_cmp);
    if values.is_empty() {
        None
    } else {
        let at = (values.len() - 1) as f64 * fraction;
        let first = at.floor() as usize;
        Some(values[first] + (values[at.ceil() as usize] - values[first]) * (at - first as f64))
    }
}
pub fn measure_pokes(input: &Input, pokes: &[PokeEvent], span_frames: u64) -> PokeStatistics {
    sample_pokes(input, pokes, span_frames).statistics
}
pub(crate) struct PokeSamples {
    pub statistics: PokeStatistics,
    pub latency: Vec<f64>,
    pub confirmation: Vec<f64>,
    pub mark_offsets: Vec<f64>,
}
pub(crate) fn sample_pokes(input: &Input, pokes: &[PokeEvent], span_frames: u64) -> PokeSamples {
    let rate = input.meta.sample_rate as f64;
    let mut labels: Vec<_> = input
        .labels
        .iter()
        .enumerate()
        .filter_map(|(index, label)| {
            let (before, after) = match label.kind.as_str() {
                "poke_truth" => (0.15, 0.15),
                "poke_mark" => (0.6, 0.2),
                _ => return None,
            };
            Some((
                label.frame.saturating_sub((before * rate).round() as u64),
                label.frame.saturating_add((after * rate).round() as u64),
                index,
            ))
        })
        .collect();
    labels.sort_unstable_by_key(|&(start, end, index)| (end, start, index));
    let mut remaining: BTreeSet<_> = pokes
        .iter()
        .enumerate()
        .map(|(index, poke)| (poke.turn_frame, index))
        .collect();
    let mut latency = vec![];
    let mut confirmation: Vec<_> = pokes
        .iter()
        .map(|poke| (poke.detect_frame as i128 - poke.turn_frame as i128) as f64 * 1000.0 / rate)
        .collect();
    let mut marks = vec![];
    let mut matched = 0;
    for (start, end, index) in &labels {
        if let Some((turn, poke_index)) = remaining
            .range((*start, 0)..=(*end, usize::MAX))
            .next()
            .copied()
        {
            remaining.remove(&(turn, poke_index));
            matched += 1;
            let label = &input.labels[*index];
            if label.kind == "poke_truth" {
                latency.push(
                    (pokes[poke_index].detect_frame as i128 - label.frame as i128) as f64 * 1000.0
                        / rate,
                );
            } else {
                marks.push((label.frame as i128 - turn as i128) as f64 * 1000.0 / rate);
            }
        }
    }
    let annotated = input.meta.source == "sim"
        || input
            .meta
            .device
            .get("annotations_complete")
            .and_then(|value| value.as_bool())
            .unwrap_or(!labels.is_empty());
    let recall = (annotated && !labels.is_empty()).then(|| matched as f64 / labels.len() as f64);
    let precision = (annotated && !pokes.is_empty()).then(|| matched as f64 / pokes.len() as f64);
    let f1 = recall.map(|r| {
        let p = precision.unwrap_or(0.0);
        if r + p > 0.0 {
            2.0 * r * p / (r + p)
        } else {
            0.0
        }
    });
    let statistics = PokeStatistics {
        detected: pokes.len(),
        labels: labels.len(),
        matched,
        unmatched_detections: pokes.len() - matched,
        false_pokes: annotated.then_some(pokes.len() - matched),
        annotations_available: annotated,
        recall,
        precision,
        f1,
        false_pokes_per_minute: (annotated && span_frames > 0)
            .then(|| (pokes.len() - matched) as f64 * 60.0 * rate / span_frames as f64),
        latency_median_ms: percentile(&mut latency, 0.5),
        latency_p95_ms: percentile(&mut latency, 0.95),
        confirmation_median_ms: percentile(&mut confirmation, 0.5),
        confirmation_p95_ms: percentile(&mut confirmation, 0.95),
        human_mark_offset_median_ms: percentile(&mut marks, 0.5),
        truth_labels: labels
            .iter()
            .filter(|(_, _, i)| input.labels[*i].kind == "poke_truth")
            .count(),
        human_marks: labels
            .iter()
            .filter(|(_, _, i)| input.labels[*i].kind == "poke_mark")
            .count(),
    };
    PokeSamples {
        statistics,
        latency,
        confirmation,
        mark_offsets: marks,
    }
}
