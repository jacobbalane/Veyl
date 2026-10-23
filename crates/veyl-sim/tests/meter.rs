use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, EngineEvent, EngineState, SoundSource,
    SourceStatus, Stream, StreamFormat, Tip,
};
use veyl_sim::{basic::quantize, scenarios, scene};

#[test]
fn meter_reports_spectrum_gaps_blocked_capture_and_silence_from_actual_pcm_for_each_seed() {
    let mut failures = vec![];
    for seed in 1..=3 {
        for name in ["dull_music", "compressed16k", "gappy", "blocked", "silence"] {
            let scene = scenarios::scene(
                name,
                seed,
                Some(if matches!(name, "blocked" | "silence") {
                    4.0
                } else {
                    40.0
                }),
            )
            .unwrap();
            let rendered = scene::render(&scene).unwrap();
            let mut engine = Engine::new(
                EngineConfig::default(),
                StreamFormat {
                    sample_rate: scene.fs,
                    ref_channels: 2,
                    mic_channels: scene.mics.len() as u8,
                },
            )
            .unwrap();
            engine
                .set_source_status(SourceStatus {
                    capture_running: true,
                    source: if name == "blocked" {
                        SoundSource::Capture
                    } else {
                        SoundSource::TestSound
                    },
                    ..SourceStatus::default()
                })
                .unwrap();
            let mut events = Vec::with_capacity(32);
            let mut readings = vec![];
            let mut pokes = 0;
            let mut gap_pokes = 0;
            for block in &rendered.blocks {
                let pcm: Vec<_> = rendered
                    .samples(block)
                    .unwrap()
                    .iter()
                    .map(|sample| quantize(*sample))
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
                for event in &events {
                    match event {
                        EngineEvent::Meter(reading) => readings.push(*reading),
                        EngineEvent::Poke(poke) => {
                            pokes += 1;
                            if (poke.detect_frame / scene.fs as u64) % 4 == 3 {
                                gap_pokes += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
            let measured: Vec<_> = readings
                .iter()
                .filter(|r| r.frame >= 5 * scene.fs as u64)
                .collect();
            let count = measured.len().max(1) as f32;
            let high = measured.iter().map(|r| r.factors.high).sum::<f32>() / count;
            let steady = measured.iter().map(|r| r.factors.steady).sum::<f32>() / count;
            let bandwidth = measured.iter().map(|r| r.bandwidth_hz).sum::<f32>() / count;
            let detail = measured.iter().filter_map(|r| r.detail_cm).sum::<f32>() / count;
            let last = engine.snapshot();
            let report = format!("{name} seed {seed}: high {high:.3}, steady {steady:.3}, bandwidth {bandwidth:.1} Hz, detail {detail:.2} cm, tip {:?}, pokes {pokes}", last.meter.tip);
            eprintln!("{report}");
            let mut priority_valid = true;
            if name == "dull_music" {
                let mut tips = std::collections::BTreeMap::new();
                for reading in &measured {
                    *tips.entry(format!("{:?}", reading.tip)).or_insert(0) += 1;
                }
                eprintln!("tip counts: {tips:?}");
                let mut previous = Tip::CaptureOff;
                let mut changed_at = 0;
                for reading in &readings {
                    if reading.tip != previous {
                        if reading.frame >= 5 * scene.fs as u64 {
                            let expected = if reading.mic_clip > 0.001 {
                                Tip::Clipping
                            } else if reading.factors.volume < 0.4 {
                                Tip::VolumeLow
                            } else if reading.factors.high < 0.3 || reading.factors.detail < 0.3 {
                                Tip::FewHighSounds
                            } else if reading.factors.steady < 0.5 {
                                Tip::Gaps
                            } else if reading.factors.clarity < 0.5 {
                                Tip::OtherSounds
                            } else {
                                Tip::Ready
                            };
                            priority_valid &= reading.tip == expected
                                && reading.frame - changed_at >= scene.fs as u64;
                        }
                        previous = reading.tip;
                        changed_at = reading.frame;
                    }
                }
            }
            let pass = match name {
                "dull_music" => high < 0.3 && priority_valid,
                "compressed16k" => {
                    (9500.0..=10500.0).contains(&bandwidth) && (1.6..=1.9).contains(&detail)
                }
                "gappy" => steady < 0.6 && gap_pokes == 0,
                "blocked" => {
                    pokes == 0
                        && readings
                            .iter()
                            .any(|r| r.tip == Tip::CaptureBlocked && r.frame <= 3 * scene.fs as u64)
                }
                "silence" => {
                    pokes == 0
                        && last.state == EngineState::WaitingForSound
                        && last.meter.tip == Tip::NothingPlaying
                }
                _ => unreachable!(),
            };
            if !pass {
                failures.push(report);
            }
            assert!(readings.iter().all(|r| r.score <= 100
                && r.mic_band_dbfs.is_finite()
                && r.bandwidth_hz.is_finite()
                && r.factors.high.is_finite()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
