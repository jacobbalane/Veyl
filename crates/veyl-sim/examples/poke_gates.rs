use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, EngineEvent, PokeEvent, Stream, StreamFormat,
};
use veyl_sim::{scenarios, scene};

// Run explicitly while M4 is in development. A nonzero exit records an unmet acceptance gate.
fn main() {
    let mut failures = vec![];
    for seed in 1..=3 {
        for name in ["poke_basic", "drift_200ppm", "pc_mode"] {
            let scene = scenarios::scene(name, seed, Some(40.0)).unwrap();
            let rendered = scene::render(&scene).unwrap();
            let config = EngineConfig {
                pc_mode: name.starts_with("pc_"),
                ..EngineConfig::default()
            };
            let mut engine = Engine::new(
                config,
                StreamFormat {
                    sample_rate: scene.fs,
                    ref_channels: 2,
                    mic_channels: scene.mics.len() as u8,
                },
            )
            .unwrap();
            engine
                .set_source_status(veyl_dsp::SourceStatus {
                    capture_running: true,
                    source: if name.starts_with("pc_") {
                        veyl_dsp::SoundSource::Pc
                    } else {
                        veyl_dsp::SoundSource::TestSound
                    },
                    ..veyl_dsp::SourceStatus::default()
                })
                .unwrap();
            let mut events = Vec::with_capacity(32);
            let mut pokes: Vec<PokeEvent> = vec![];
            for block in &rendered.blocks {
                let pcm: Vec<_> = rendered
                    .samples(block)
                    .unwrap()
                    .iter()
                    .map(|sample| veyl_sim::basic::quantize(*sample))
                    .collect();
                engine
                    .push(BlockIn {
                        stream: if block.stream == 0 {
                            Stream::Reference
                        } else {
                            Stream::Microphone
                        },
                        first_frame: block.first_frame,
                        capture_ns: Some(block.capture_ns),
                        flags: BlockFlags(block.flags),
                        pre_rms_dbfs: block.pre_rms_dbfs,
                        pre_peak: block.pre_peak,
                        samples: &pcm,
                    })
                    .unwrap();
                events.clear();
                engine.process(&mut events).unwrap();
                pokes.extend(events.iter().filter_map(|event| {
                    if let EngineEvent::Poke(poke) = event {
                        Some(*poke)
                    } else {
                        None
                    }
                }));
            }
            let mut matched = vec![false; rendered.labels.len()];
            let mut latencies = vec![];
            for poke in &pokes {
                if let Some((index, _)) = rendered
                    .labels
                    .iter()
                    .enumerate()
                    .filter(|(i, label)| {
                        !matched[*i]
                            && label.turn_frame.abs_diff(poke.turn_frame)
                                <= (scene.fs as f64 * 0.15) as u64
                    })
                    .min_by_key(|(_, label)| label.turn_frame.abs_diff(poke.turn_frame))
                {
                    matched[index] = true;
                    latencies.push(
                        (poke.detect_frame - poke.turn_frame) as f32 * 1000.0 / scene.fs as f32,
                    );
                }
            }
            let hits = matched.iter().filter(|found| **found).count();
            let recall = hits as f32 / rendered.labels.len() as f32;
            let precision = hits as f32 / pokes.len().max(1) as f32;
            latencies.sort_unstable_by(f32::total_cmp);
            let median = latencies
                .get(latencies.len() / 2)
                .copied()
                .unwrap_or(f32::INFINITY);
            let report = format!("{name} seed {seed}: {hits}/{} matched, {} detections, recall {recall:.2}, precision {precision:.2}, median confirmation {median:.1} ms", rendered.labels.len(), pokes.len());
            eprintln!("{report}");
            if seed == 1 {
                eprintln!("detections: {pokes:?}");
            }
            let required = if name == "pc_mode" { 0.9 } else { 0.95 };
            if recall < required || precision < 0.95 || name == "poke_basic" && median > 80.0 {
                failures.push(report);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
