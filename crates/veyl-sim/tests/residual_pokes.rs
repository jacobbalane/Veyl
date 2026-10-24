//! Residual-motion candidate regressions (decision 0020). These are narrower than the complete
//! M4 gesture acceptance gates.
use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, EngineEvent, PokeEvent, SoundSource,
    SourceStatus, Stream, StreamFormat,
};
use veyl_sim::{basic::quantize, scenarios, scene, Rendered};

fn candidate() -> EngineConfig {
    EngineConfig {
        infer_single_path: true,
        resample_reference: true,
        residual_motion: true,
        floor_relative: 1e-8,
        impulse_ratio: 12.0,
        ..EngineConfig::default()
    }
}

fn run(rendered: &Rendered) -> (Vec<PokeEvent>, u64) {
    let mut engine = Engine::new(
        candidate(),
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
            source: SoundSource::TestSound,
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
fn residual_candidate_recovers_stereo_and_compressed_pokes_for_each_seed_with_no_extras() {
    for name in ["stereo_speakers", "compressed16k"] {
        for seed in 1..=3 {
            let rendered = scene::render(&scenarios::scene(name, seed, None).unwrap()).unwrap();
            let (pokes, jumps) = run(&rendered);
            let (matched, extra) = matches(&rendered, &pokes);
            assert_eq!(
                (matched, extra),
                (rendered.labels.len(), 0),
                "{name} seed {seed}: {matched} matched, {extra} extra"
            );
            assert_eq!(jumps, 0, "{name} seed {seed}: {jumps} model jumps");
        }
    }
}
