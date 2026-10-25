//! Plan §6.14: a 60-second scenario processes in under two seconds in release. Wall-clock time
//! depends on the machine, so the test is ignored by default and run deliberately, on the
//! target hardware, with `cargo test --release -p veyl-sim --test performance -- --ignored`.
use std::time::Instant;
use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, SoundSource, SourceStatus, Stream,
    StreamFormat,
};
use veyl_sim::{basic::quantize, scenarios, scene};

#[test]
#[ignore = "wall-clock measurement for the target hardware"]
fn sixty_seconds_of_audio_process_in_under_two_seconds() {
    for name in ["poke_basic", "pc_mode"] {
        let rendered = scene::render(&scenarios::scene(name, 1, Some(60.0)).unwrap()).unwrap();
        let pcm: [Vec<i16>; 2] = [
            rendered.reference.iter().map(|v| quantize(*v)).collect(),
            rendered.mic.iter().map(|v| quantize(*v)).collect(),
        ];
        let mut engine = Engine::new(
            EngineConfig {
                pc_mode: name == "pc_mode",
                ..EngineConfig::default()
            },
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
                source: if name == "pc_mode" {
                    SoundSource::Pc
                } else {
                    SoundSource::TestSound
                },
                ..SourceStatus::default()
            })
            .unwrap();
        let mut events = Vec::with_capacity(64);
        let start = Instant::now();
        for meta in &rendered.blocks {
            let channels = if meta.stream == 0 {
                rendered.ref_channels
            } else {
                rendered.mic_channels
            } as usize;
            let first = meta.first_frame as usize * channels;
            engine
                .push(BlockIn {
                    stream: if meta.stream == 0 {
                        Stream::Reference
                    } else {
                        Stream::Microphone
                    },
                    first_frame: meta.first_frame,
                    capture_ns: Some(meta.capture_ns),
                    flags: BlockFlags(meta.flags),
                    pre_rms_dbfs: meta.pre_rms_dbfs,
                    pre_peak: meta.pre_peak,
                    samples: &pcm[meta.stream as usize]
                        [first..first + rendered.sample_rate as usize / 100 * channels],
                })
                .unwrap();
            events.clear();
            engine.process(&mut events).unwrap();
        }
        let seconds = start.elapsed().as_secs_f64();
        eprintln!("{name}: 60 s of audio in {seconds:.2} s");
        assert!(seconds < 2.0, "{name}: 60 s of audio took {seconds:.2} s");
    }
}
