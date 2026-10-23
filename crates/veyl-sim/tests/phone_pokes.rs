//! This candidate regression is narrower than the complete M4 gesture acceptance gates.
use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, EngineEvent, PokeEvent, SoundRoute,
    SoundSource, SourceStatus, Stream, StreamFormat,
};
use veyl_sim::{basic::quantize, scenarios, scene, Rendered};

fn run(
    rendered: &Rendered,
    status: SourceStatus,
    pc: bool,
    paused: bool,
) -> (Vec<PokeEvent>, usize) {
    let config = EngineConfig {
        pc_mode: pc,
        fast_seconds: 0.005,
        infer_single_path: true,
        ..EngineConfig::default()
    };
    let mut engine = Engine::new(
        config,
        StreamFormat {
            sample_rate: 48000,
            ref_channels: 2,
            mic_channels: 2,
        },
    )
    .unwrap();
    engine.set_source_status(status).unwrap();
    engine.set_paused(paused);
    let mut events = Vec::with_capacity(32);
    let mut pokes = vec![];
    let mut hands = 0;
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
                EngineEvent::Poke(poke) => {
                    assert!(!engine.snapshot().suspended);
                    pokes.push(*poke);
                }
                EngineEvent::Hand { present: true, .. } => hands += 1,
                _ => {}
            }
        }
    }
    (pokes, hands)
}

#[test]
fn single_speaker_candidate_recovers_all_basic_pokes_for_each_seed_with_no_extras() {
    for seed in 1..=3 {
        let scene = scenarios::scene("poke_basic", seed, None).unwrap();
        let rendered = scene::render(&scene).unwrap();
        let (pokes, _) = run(
            &rendered,
            SourceStatus {
                capture_running: true,
                source: SoundSource::TestSound,
                ..SourceStatus::default()
            },
            false,
            false,
        );
        assert_eq!(
            pokes.len(),
            10,
            "seed {seed}: detections differ from the ten truth pokes"
        );
        let mut matched = vec![false; rendered.labels.len()];
        let mut latency = vec![];
        for poke in &pokes {
            let index = rendered
                .labels
                .iter()
                .enumerate()
                .filter(|(i, label)| {
                    !matched[*i] && label.turn_frame.abs_diff(poke.turn_frame) <= 7200
                })
                .min_by_key(|(_, label)| label.turn_frame.abs_diff(poke.turn_frame))
                .map(|(i, _)| i)
                .expect("A detected poke has no nearby unused truth label.");
            matched[index] = true;
            latency.push(poke.detect_frame - poke.turn_frame);
        }
        assert!(matched.iter().all(|hit| *hit));
        latency.sort_unstable();
        assert!(
            latency[latency.len() / 2] <= 3840,
            "Median confirmation exceeds 80 ms."
        );
    }
}

#[test]
fn off_external_silenced_headphone_and_paused_sources_cannot_confirm_visible_hand_motion() {
    let scene = scenarios::scene("poke_basic", 1, Some(8.0)).unwrap();
    let rendered = scene::render(&scene).unwrap();
    let running = SourceStatus {
        capture_running: true,
        source: SoundSource::TestSound,
        ..SourceStatus::default()
    };
    for (status, pc, paused) in [
        (SourceStatus::default(), false, false),
        (
            SourceStatus {
                route: SoundRoute::External,
                ..running
            },
            false,
            false,
        ),
        (
            SourceStatus {
                mic_silenced: true,
                ..running
            },
            false,
            false,
        ),
        (
            SourceStatus {
                route: SoundRoute::Headphones,
                ..running
            },
            true,
            false,
        ),
        (running, false, true),
    ] {
        let (pokes, hands) = run(&rendered, status, pc, paused);
        assert!(
            hands > 0,
            "Safety test never presented real acoustic hand features."
        );
        assert!(pokes.is_empty());
    }
}
