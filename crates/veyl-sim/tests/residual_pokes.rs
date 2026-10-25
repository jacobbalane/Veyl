//! Default engine regressions on scenes it must keep (decisions 0020–0023). The complete gates
//! are in veyl-lab's scenario_gates test.
use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, EngineEvent, PokeEvent, SoundSource,
    SourceStatus, Stream, StreamFormat,
};
use veyl_sim::{basic::quantize, scenarios, scene, Rendered};

fn candidate(pc: bool) -> EngineConfig {
    EngineConfig {
        pc_mode: pc,
        range_max_cm: if pc { 55.0 } else { 40.0 },
        ..EngineConfig::default()
    }
}

fn run(rendered: &Rendered, pc: bool) -> (Vec<PokeEvent>, u64) {
    let mut engine = Engine::new(
        candidate(pc),
        StreamFormat {
            sample_rate: rendered.sample_rate,
            ref_channels: rendered.ref_channels,
            mic_channels: rendered.mic_channels,
        },
    )
    .unwrap();
    engine
        .set_source_status(SourceStatus {
            capture_running: true,
            source: if pc {
                SoundSource::Pc
            } else {
                SoundSource::TestSound
            },
            ..SourceStatus::default()
        })
        .unwrap();
    let mut events = Vec::with_capacity(32);
    let mut pokes = vec![];
    let mut jumps = 0;
    for block in &rendered.blocks {
        let pcm: Vec<_> = rendered
            .samples(block)
            .unwrap()
            .iter()
            .map(|v| quantize(*v))
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
                EngineEvent::Poke(poke) => pokes.push(*poke),
                EngineEvent::Warning(veyl_dsp::Warning::ModelJump) => jumps += 1,
                _ => {}
            }
        }
    }
    (pokes, jumps)
}

/// One-to-one matches within the simulator's truth window of 0.15 s.
fn matches(rendered: &Rendered, pokes: &[PokeEvent]) -> (usize, usize) {
    let window = (0.15 * rendered.sample_rate as f64) as u64;
    let mut used = vec![false; rendered.labels.len()];
    let mut matched = 0;
    for poke in pokes {
        if let Some(index) = (0..rendered.labels.len())
            .filter(|&i| {
                !used[i] && rendered.labels[i].turn_frame.abs_diff(poke.turn_frame) <= window
            })
            .min_by_key(|&i| rendered.labels[i].turn_frame.abs_diff(poke.turn_frame))
        {
            used[index] = true;
            matched += 1;
        }
    }
    (matched, pokes.len() - matched)
}

#[test]
fn short_change_candidate_recovers_stereo_compressed_and_pc_pokes_for_each_seed() {
    for name in ["stereo_speakers", "compressed16k", "pc_mode"] {
        let pc = name.starts_with("pc_");
        for seed in 1..=3 {
            let rendered = scene::render(&scenarios::scene(name, seed, None).unwrap()).unwrap();
            let (pokes, jumps) = run(&rendered, pc);
            let (matched, extra) = matches(&rendered, &pokes);
            // PC's own gate allows one miss in ten; the phone scenes match every poke.
            let required = if pc {
                rendered.labels.len() * 9 / 10
            } else {
                rendered.labels.len()
            };
            assert!(
                matched >= required && extra == 0,
                "{name} seed {seed}: {matched} matched, {extra} extra"
            );
            assert_eq!(jumps, 0, "{name} seed {seed}: {jumps} model jumps");
        }
    }
}
