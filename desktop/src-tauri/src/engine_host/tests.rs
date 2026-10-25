use super::*;
use crate::source::SoundSource as _;
use std::time::Duration;
fn format(run: u64, rate: u32) -> AudioConfig {
    AudioConfig {
        run_id: run,
        source: "test_sound".into(),
        sample_rate: rate,
        frames_per_block: (rate / 100) as u16,
        ref_channels: 2,
        mic_channels: 1,
        mic_source: "unprocessed".into(),
        effects_off: vec![],
        hp_cutoff_hz: 8000,
        mics: vec![],
        sound_check: None,
    }
}
fn running() -> Capture {
    Capture {
        state: "running".into(),
        source: "test_sound".into(),
        route: "speaker".into(),
        reason: None,
    }
}
fn block(seq: u32, rate: u32, sample: i16) -> AudioBlock {
    AudioBlock {
        stream: 1,
        channels: 1,
        sample_rate: rate,
        seq,
        first_frame: seq as u64 * (rate / 100) as u64,
        capture_ns: i64::MIN,
        frames: (rate / 100) as u16,
        flags: 0,
        pre_rms_dbfs: -12.0,
        pre_peak: 0.5,
        samples: vec![sample; (rate / 100) as usize],
    }
}
fn wait_for(host: &EngineHost, predicate: impl Fn(&LevelsView) -> bool) -> LevelsView {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let view = host.levels();
        if predicate(&view) {
            return view;
        }
        assert!(Instant::now() < deadline, "Engine did not drain its queue.");
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[tokio::test]
async fn levels_count_gaps_and_reject_old_sessions_and_stopped_audio() {
    let (host, source) = EngineHost::start();
    source.configure(1, format(7, 48000)).await.unwrap();
    source.status(1, running()).await.unwrap();
    source.audio(1, block(0, 48000, 16384), Some(15.0)).unwrap();
    source.audio(0, block(1, 48000, 16384), None).unwrap();
    source.audio(1, block(2, 48000, 16384), Some(15.0)).unwrap();
    let first = wait_for(&host, |view| view.microphone.blocks == 2);
    assert_eq!(first.microphone.gaps, 1);
    assert_eq!(first.microphone.latest_frame, Some(1439));
    assert!((first.microphone.filtered_dbfs + 6.0206).abs() < 0.001);
    assert_eq!(first.arrival_ms, Some(15.0));
    assert_eq!(first.reference.blocks, 0);
    source.configure(2, format(8, 44100)).await.unwrap();
    source.status(2, running()).await.unwrap();
    source.audio(1, block(3, 48000, 16384), None).unwrap();
    source.audio(2, block(0, 44100, 0), None).unwrap();
    let second = wait_for(&host, |view| {
        view.run_id == Some(8) && view.microphone.blocks == 1
    });
    assert_eq!(second.sample_rate, Some(44100));
    assert_eq!(second.microphone.filtered_dbfs, -120.0);
    assert_eq!(second.microphone.gaps, 0);
    assert!(source.configure(1, format(7, 48000)).await.is_err());
    assert!(!source.audio(1, block(4, 48000, 1), None).unwrap());
    assert_eq!(host.levels().run_id, Some(8));
    source
        .status(
            2,
            Capture {
                state: "idle".into(),
                reason: Some("user".into()),
                ..running()
            },
        )
        .await
        .unwrap();
    source.audio(2, block(1, 44100, 1), None).unwrap();
    wait_for(&host, |view| !view.running);
    source.disconnected(2).await.unwrap();
    assert_eq!(host.levels().microphone.blocks, 1);
}
#[test]
fn capture_states_map_to_the_engines_source_status() {
    let status = source_status(&running(), Some(0.4));
    assert!(status.capture_running && status.mic_available && !status.mic_silenced);
    assert_eq!(status.source, SoundSource::TestSound);
    assert_eq!(status.route, SoundRoute::Speaker);
    assert_eq!(status.media_volume, Some(0.4));
    let silenced = source_status(
        &Capture {
            source: "capture".into(),
            route: "external".into(),
            reason: Some("mic_silenced".into()),
            ..running()
        },
        None,
    );
    assert!(silenced.mic_silenced && !silenced.permits_detection(false));
    assert_eq!(silenced.source, SoundSource::Capture);
    assert_eq!(silenced.route, SoundRoute::External);
    let pc = source_status(
        &Capture {
            source: "pc".into(),
            route: "bluetooth".into(),
            ..running()
        },
        None,
    );
    assert_eq!(
        (pc.source, pc.route),
        (SoundSource::Pc, SoundRoute::Bluetooth)
    );
    assert!(pc.permits_detection(true));
    let stopped = source_status(
        &Capture {
            state: "error".into(),
            reason: Some("mic_error".into()),
            ..running()
        },
        None,
    );
    assert!(!stopped.capture_running && !stopped.mic_available);
    assert_eq!(name(Tip::FewHighSounds), "few_high_sounds");
    assert_eq!(name(Warning::ModelJump), "model_jump");
}
#[test]
fn latency_keeps_the_last_fifty_pokes() {
    let mut values: VecDeque<f64> = (1..=60).map(f64::from).collect();
    while values.len() > LATENCY_POKES {
        values.pop_front();
    }
    let view = latency_view(&values);
    assert_eq!(view.pokes, 50);
    assert_eq!(view.median_ms, Some(36.0));
    assert_eq!(view.p95_ms, Some(58.0));
    assert_eq!(latency_view(&VecDeque::new()).median_ms, None);
}
#[derive(Default)]
struct Seen {
    engine: Vec<EngineView>,
    gestures: Vec<GestureEvent>,
    frames: Vec<LiveFrame>,
}
/// A simulated phone scene through the host, as the connection worker delivers it.
#[tokio::test(flavor = "multi_thread")]
async fn simulated_pokes_reach_the_interface_and_the_phone() {
    let seconds = 16.0;
    let input = veyl_lab::recording::Input::generated(veyl_sim::poke_basic(1, seconds).unwrap());
    let seen = Arc::new(Mutex::new(Seen::default()));
    let (phone, mut phone_rx) = mpsc::channel(4096);
    let observer: Observer = {
        let seen = seen.clone();
        Arc::new(move |notice| match notice {
            Notice::Engine(view) => seen.lock().unwrap().engine.push(view),
            Notice::Gesture(gesture) => seen.lock().unwrap().gestures.push(gesture),
            _ => {}
        })
    };
    let (host, source) = EngineHost::launch(HostOptions {
        observer: Some(observer),
        phone: Some(phone),
        ..Default::default()
    });
    {
        let seen = seen.clone();
        host.subscribe(Box::new(move |frame| {
            seen.lock().unwrap().frames.push(frame.clone());
            true
        }));
    }
    let generation = source.next_generation();
    let meta = &input.meta;
    let mut config = format(1, meta.sample_rate);
    config.ref_channels = meta.ref_channels;
    config.mic_channels = meta.mic_channels;
    source.configure(generation, config).await.unwrap();
    source.status(generation, running()).await.unwrap();
    source.volume(generation, 0.7);
    let started = Instant::now();
    for index in 0..input.blocks.len() {
        let block = input.block(index, 0).unwrap();
        host.sender
            .send(EngineInput::Audio(
                generation,
                block,
                Some(20.0),
                Instant::now(),
            ))
            .await
            .unwrap();
    }
    host.apply(EngineSettings::default()).await.unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    while host.levels().microphone.blocks < input.blocks.len() as u64 / 2 {
        assert!(Instant::now() < deadline, "Engine did not drain its queue.");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    eprintln!(
        "{seconds} s of audio took {:.1} s",
        started.elapsed().as_secs_f64()
    );
    let seen = std::mem::take(&mut *seen.lock().unwrap());
    // About 30 live frames per second of microphone audio, each with one row per centimetre.
    let frames = seen.frames.len() as f32;
    assert!((frames - 30.0 * seconds).abs() <= 2.0, "{frames} frames");
    assert!(seen.frames.iter().all(|frame| frame.motion.len() == 60
        && frame
            .motion
            .iter()
            .all(|value| (0.0..=30.0).contains(value))));
    assert!(seen
        .frames
        .iter()
        .any(|frame| frame.motion.iter().any(|value| *value > 0.0)));
    let meters = seen
        .frames
        .iter()
        .filter(|frame| frame.meter.is_some())
        .count() as f32;
    assert!(
        (meters - 10.0 * seconds).abs() <= 0.25 * 10.0 * seconds,
        "{meters} meters"
    );
    // The engine view announces the running state.
    let states: Vec<_> = seen.engine.iter().map(|view| view.state).collect();
    assert!(states.contains(&EngineState::Running), "{states:?}");
    assert_eq!(host.engine().state, EngineState::Running);
    assert!(host.engine().meter.is_some_and(|meter| meter.score > 0));
    // Pokes reach the interface with sensible values and as messages to the phone.
    assert!(!seen.gestures.is_empty(), "no pokes");
    assert!(seen.gestures.iter().all(|gesture| gesture.kind == "poke"
        && gesture.test_only
        && !gesture.acted
        && (4.0..=40.0).contains(&gesture.distance_cm)));
    let (mut meters_sent, mut gestures_sent) = (0, 0);
    while let Ok(message) = phone_rx.try_recv() {
        match message {
            Control::Meter { label, tip, .. } => {
                assert!(!label.is_empty() && !tip.is_empty());
                meters_sent += 1;
            }
            Control::Gesture { kind, acted } => {
                assert!(kind == "poke" && !acted);
                gestures_sent += 1;
            }
            _ => panic!("unexpected message"),
        }
    }
    assert_eq!(gestures_sent, seen.gestures.len());
    // The audio arrived faster than real time, so the 5 Hz limit sends fewer meters than frames.
    assert!(meters_sent >= 1 && meters_sent as f32 <= meters);
    let latency = host.levels().latency;
    assert_eq!(latency.pokes as usize, seen.gestures.len());
    source.disconnected(generation).await.unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while host.engine().state != EngineState::NoAudio {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
