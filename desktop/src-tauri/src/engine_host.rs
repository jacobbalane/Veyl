//! The engine thread (§7.6): transport levels for Diagnostics, and the echo engine for the
//! interface and the phone. It stays thin so the lab's end-to-end test can make the same calls.
use crate::{
    actions::ActionsGate,
    connection::Observer,
    events::{EngineView, GestureEvent, HandView, LiveFrame, MeterView, Notice},
    source::PhoneSource,
};
use serde::Serialize;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use veyl_dsp::{
    config::EngineConfig, BlockFlags, BlockIn, Engine, EngineEvent, EngineSnapshot, EngineState,
    RangePreset, Sensitivity, SoundRoute, SoundSource, SourceStatus, Stream, StreamFormat, Tip,
    Warning,
};
use veyl_link::{
    audio::AudioBlock,
    messages::{AudioConfig, Capture, Control},
};

pub(crate) enum EngineInput {
    Config(u64, AudioConfig),
    Status(u64, Capture),
    Volume(u64, f32),
    /// A block, its transport delay in ms when known, and when the source handed it over.
    Audio(u64, AudioBlock, Option<f64>, Instant),
    Disconnected(u64),
    Settings(EngineSettings),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngineSettings {
    pub sensitivity: Sensitivity,
    pub range: RangePreset,
    pub temperature_c: f32,
}
impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            sensitivity: Sensitivity::Normal,
            range: RangePreset::Normal,
            temperature_c: 20.0,
        }
    }
}
impl EngineSettings {
    pub(crate) fn from_settings(settings: &crate::settings::Settings) -> Self {
        Self {
            sensitivity: match settings.sensitivity.as_str() {
                "low" => Sensitivity::Low,
                "high" => Sensitivity::High,
                _ => Sensitivity::Normal,
            },
            range: match settings.range.as_str() {
                "near" => RangePreset::Near,
                "far" => RangePreset::Far,
                _ => RangePreset::Normal,
            },
            temperature_c: if settings.temperature_c.is_finite() {
                settings.temperature_c.clamp(-10.0, 45.0)
            } else {
                20.0
            },
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamLevels {
    pub blocks: u64,
    pub blocks_per_second: f64,
    pub pre_dbfs: f32,
    pub filtered_dbfs: f32,
    pub peak: f32,
    pub gaps: u64,
    pub latest_frame: Option<u64>,
}
impl Default for StreamLevels {
    fn default() -> Self {
        Self {
            blocks: 0,
            blocks_per_second: 0.0,
            pre_dbfs: -120.0,
            filtered_dbfs: -120.0,
            peak: 0.0,
            gaps: 0,
            latest_frame: None,
        }
    }
}
/// Time from the poke's turning point to its detection on the PC, over the last 50 pokes.
#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LatencyView {
    pub pokes: u32,
    pub median_ms: Option<f64>,
    pub p95_ms: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LevelsView {
    pub run_id: Option<u64>,
    pub reference: StreamLevels,
    pub microphone: StreamLevels,
    pub sample_rate: Option<u32>,
    pub engine_drops: u64,
    pub arrival_ms: Option<f64>,
    pub running: bool,
    pub latency: LatencyView,
}
/// Receives each live frame; returning false unsubscribes it.
pub type LiveSink = Box<dyn FnMut(&LiveFrame) -> bool + Send>;
#[derive(Default)]
pub struct HostOptions {
    pub recorder: Option<crate::recorder::Recorder>,
    pub settings: EngineSettings,
    pub observer: Option<Observer>,
    /// CONTROL messages for the connected phone: the meter and gestures.
    pub phone: Option<mpsc::Sender<Control>>,
    pub actions: Option<Arc<ActionsGate>>,
}
/// The poke test counts up to 10 pokes within 20 s; actions never run meanwhile.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PokeTestResult {
    pub seen: u32,
    pub of: u32,
}
struct PokeTest {
    started: Instant,
    seen: u32,
}
const POKE_TEST_POKES: u32 = 10;
const POKE_TEST_TIME: Duration = Duration::from_secs(20);
pub struct EngineHost {
    view: Arc<Mutex<LevelsView>>,
    engine: Arc<Mutex<EngineView>>,
    live: Arc<Mutex<Option<LiveSink>>>,
    test: Arc<Mutex<Option<PokeTest>>>,
    actions: Option<Arc<ActionsGate>>,
    dropped: Arc<AtomicU64>,
    sender: mpsc::Sender<EngineInput>,
}
const LIVE_PER_SECOND: u64 = 30;
const PHONE_METER_INTERVAL: Duration = Duration::from_millis(200);
const LATENCY_POKES: usize = 50;
/// Reference blocks without a microphone block before the microphone counts as silent: 2 s.
const MICROPHONE_SILENT_BLOCKS: u32 = 200;
/// A warning stays in the engine view for this many seconds of audio.
const WARNING_SECONDS: u64 = 5;
impl EngineHost {
    pub fn start() -> (Self, PhoneSource) {
        Self::launch(HostOptions::default())
    }
    pub fn with_recorder(recorder: Option<crate::recorder::Recorder>) -> (Self, PhoneSource) {
        Self::launch(HostOptions {
            recorder,
            ..Default::default()
        })
    }
    pub fn launch(options: HostOptions) -> (Self, PhoneSource) {
        let (sender, mut receiver) = mpsc::channel(400);
        let dropped = Arc::new(AtomicU64::new(0));
        let source = PhoneSource::with_recorder(sender.clone(), dropped.clone(), options.recorder);
        let host = Self {
            view: Arc::default(),
            engine: Arc::default(),
            live: Arc::default(),
            test: Arc::default(),
            actions: options.actions.clone(),
            dropped,
            sender,
        };
        let mut worker = Worker {
            levels: host.view.clone(),
            shared: host.engine.clone(),
            live: host.live.clone(),
            test: host.test.clone(),
            actions: options.actions,
            observer: options.observer,
            phone: options.phone,
            settings: options.settings,
            origin: Instant::now(),
            generation: 0,
            format: None,
            capture: None,
            volume: None,
            silent: (0, false),
            expected: [None, None],
            window: [Instant::now(); 2],
            counts: [0; 2],
            session: None,
            latency: VecDeque::new(),
            events: Vec::new(),
        };
        std::thread::Builder::new()
            .name("Veyl-engine".into())
            .spawn(move || {
                while let Some(message) = receiver.blocking_recv() {
                    worker.handle(message);
                }
            })
            .expect("Engine worker could not start.");
        (host, source)
    }
    pub fn levels(&self) -> LevelsView {
        let mut view = self
            .view
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        view.engine_drops = self.dropped.load(Ordering::Relaxed);
        view
    }
    pub fn engine(&self) -> EngineView {
        self.engine
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    /// Replaces the previous subscriber, as a reloaded window registers a new channel.
    pub fn subscribe(&self, sink: LiveSink) {
        *self.live.lock().unwrap_or_else(|error| error.into_inner()) = Some(sink);
    }
    /// Starts the poke test or the onboarding practice; actions stay off until it stops.
    pub fn start_test(&self) {
        if let Some(actions) = &self.actions {
            actions.set_testing(true);
        }
        *self.test.lock().unwrap_or_else(|error| error.into_inner()) = Some(PokeTest {
            started: Instant::now(),
            seen: 0,
        });
    }
    pub fn stop_test(&self) -> PokeTestResult {
        let test = self
            .test
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(actions) = &self.actions {
            actions.set_testing(false);
        }
        PokeTestResult {
            seen: test.map_or(0, |test| test.seen),
            of: POKE_TEST_POKES,
        }
    }
    pub async fn apply(&self, settings: EngineSettings) -> Result<(), String> {
        tokio::time::timeout(
            Duration::from_secs(1),
            self.sender.send(EngineInput::Settings(settings)),
        )
        .await
        .map_err(|_| "engine_busy")?
        .map_err(|_| "engine_stopped".into())
    }
}
struct Session {
    engine: Engine,
    rate: u32,
    pc: bool,
    /// The first microphone frame and the number of live frames sent since.
    live: Option<(u64, u64)>,
    meter: Option<MeterView>,
    meter_pending: bool,
    phone_meter_at: Option<Instant>,
    /// The latest microphone block's first frame and when it was captured, on this PC's clock.
    anchor: Option<(u64, Instant)>,
    /// The latest microphone frame received.
    frame: u64,
    state: EngineState,
    suspended: Option<String>,
    warning: Option<(Warning, u64)>,
    hand: HandView,
}
struct Worker {
    levels: Arc<Mutex<LevelsView>>,
    shared: Arc<Mutex<EngineView>>,
    live: Arc<Mutex<Option<LiveSink>>>,
    test: Arc<Mutex<Option<PokeTest>>>,
    actions: Option<Arc<ActionsGate>>,
    observer: Option<Observer>,
    phone: Option<mpsc::Sender<Control>>,
    settings: EngineSettings,
    origin: Instant,
    generation: u64,
    format: Option<AudioConfig>,
    capture: Option<(u64, Capture)>,
    volume: Option<f32>,
    /// Reference blocks since the last microphone block, and whether that made the microphone
    /// count as silent: a microphone that delivers nothing (another app holds it, or a virtual
    /// device whose software is closed) is not low volume.
    silent: (u32, bool),
    expected: [Option<u64>; 2],
    window: [Instant; 2],
    counts: [u64; 2],
    session: Option<Session>,
    latency: VecDeque<f64>,
    events: Vec<EngineEvent>,
}
impl Worker {
    fn handle(&mut self, message: EngineInput) {
        match message {
            EngineInput::Config(next, config) if next >= self.generation => {
                self.generation = next;
                self.expected = [None, None];
                self.window = [Instant::now(); 2];
                self.counts = [0; 2];
                self.volume = None;
                self.silent = (0, false);
                {
                    let mut levels = self.lock_levels();
                    *levels = LevelsView {
                        run_id: Some(config.run_id),
                        sample_rate: Some(config.sample_rate),
                        latency: std::mem::take(&mut levels.latency),
                        ..Default::default()
                    };
                }
                self.format = Some(config);
                self.capture = self.capture.take().filter(|(at, _)| *at == next);
                self.open();
            }
            EngineInput::Status(current, capture) if current >= self.generation => {
                self.capture = Some((current, capture));
                if current == self.generation {
                    self.status();
                }
            }
            EngineInput::Volume(current, volume) if current == self.generation => {
                self.volume = volume.is_finite().then(|| volume.clamp(0.0, 1.0));
                self.status();
            }
            EngineInput::Disconnected(current) if current == self.generation => {
                {
                    let mut levels = self.lock_levels();
                    levels.running = false;
                    levels.reference.blocks_per_second = 0.0;
                    levels.microphone.blocks_per_second = 0.0;
                }
                self.capture = None;
                self.session = None;
                self.publish(EngineView::default(), true);
            }
            EngineInput::Audio(current, block, arrival, sent) if current == self.generation => {
                if self.lock_levels().running {
                    self.audio(block, arrival, sent);
                }
            }
            EngineInput::Settings(settings) => self.apply(settings),
            _ => {}
        }
    }
    fn lock_levels(&self) -> std::sync::MutexGuard<'_, LevelsView> {
        self.levels
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
    /// Starts a new engine for the current format and applies the latest source status.
    fn open(&mut self) {
        self.session = None;
        if let Some(config) = &self.format {
            let pc = config.source == "pc";
            let engine_config = EngineConfig {
                pc_mode: pc,
                f_lo: config.hp_cutoff_hz as f32,
                range_max_cm: if pc { 55.0 } else { 40.0 },
                temperature_c: self.settings.temperature_c,
                ..EngineConfig::default()
            };
            let format = StreamFormat {
                sample_rate: config.sample_rate,
                ref_channels: config.ref_channels,
                mic_channels: config.mic_channels,
            };
            match Engine::new(engine_config, format) {
                Ok(mut engine) => {
                    engine.set_sensitivity(self.settings.sensitivity);
                    engine.set_range(self.settings.range);
                    self.session = Some(Session {
                        engine,
                        rate: config.sample_rate,
                        pc,
                        live: None,
                        meter: None,
                        meter_pending: false,
                        phone_meter_at: None,
                        anchor: None,
                        frame: 0,
                        state: EngineState::NoAudio,
                        suspended: None,
                        warning: None,
                        hand: HandView::default(),
                    });
                }
                Err(error) => tracing::warn!("The echo engine could not start: {error}"),
            }
        }
        self.publish(EngineView::default(), true);
        self.status();
    }
    /// Applies the latest capture state and media volume of the current generation.
    fn status(&mut self) {
        let Some((current, capture)) = &self.capture else {
            return;
        };
        if *current != self.generation {
            return;
        }
        let running = capture.state == "running";
        let mut status = source_status(capture, self.volume);
        status.mic_silenced |= self.silent.1;
        {
            let mut levels = self.lock_levels();
            levels.running = running;
            if !running {
                levels.reference.blocks_per_second = 0.0;
                levels.microphone.blocks_per_second = 0.0;
            }
        }
        if let Some(session) = &mut self.session {
            if session.engine.set_source_status(status).is_err() {
                tracing::warn!("The engine rejected a source status.");
            }
            if !running {
                // Audio resumes with new frame numbers; old buffers and the background are stale.
                session.engine.reset();
                session.live = None;
                session.anchor = None;
                session.state = EngineState::NoAudio;
                session.suspended = None;
                session.warning = None;
                session.hand = HandView::default();
            }
        }
        self.refresh(false);
    }
    fn apply(&mut self, settings: EngineSettings) {
        let previous = std::mem::replace(&mut self.settings, settings);
        if previous.temperature_c != settings.temperature_c {
            // The speed of sound sets the distance scale, which the engine fixes at creation.
            if self.session.is_some() {
                self.open();
            }
        } else if let Some(session) = &mut self.session {
            session.engine.set_sensitivity(settings.sensitivity);
            session.engine.set_range(settings.range);
        }
    }
    fn audio(&mut self, block: AudioBlock, arrival: Option<f64>, sent: Instant) {
        let Some(config) = &self.format else {
            return;
        };
        let index = block.stream as usize;
        if index > 1
            || block.sample_rate != config.sample_rate
            || block.frames != config.frames_per_block
            || block.channels
                != if index == 0 {
                    config.ref_channels
                } else {
                    config.mic_channels
                }
        {
            return;
        }
        let input = input(&block);
        let Some(end) = input.first_frame.checked_add(block.frames as u64) else {
            return;
        };
        self.measure(&input, index, end, arrival);
        let blocks = if index == 0 { self.silent.0 + 1 } else { 0 };
        let silent = blocks >= MICROPHONE_SILENT_BLOCKS;
        self.silent.0 = blocks.min(MICROPHONE_SILENT_BLOCKS);
        if silent != self.silent.1 {
            self.silent.1 = silent;
            self.status();
        }
        let Some(session) = &mut self.session else {
            return;
        };
        if input.stream == Stream::Microphone {
            // Phone blocks carry their transport delay; PC blocks reach the queue as soon as they
            // are complete.
            let duration = Duration::from_secs_f64(block.frames as f64 / session.rate as f64);
            let captured = match arrival {
                Some(ms) if (0.0..10_000.0).contains(&ms) => {
                    sent.checked_sub(Duration::from_secs_f64(ms / 1000.0))
                }
                _ if session.pc => sent.checked_sub(duration),
                _ => None,
            };
            session.anchor = captured.map(|at| (input.first_frame, at));
            session.frame = session.frame.max(end);
        }
        if session.engine.push(input).is_err() {
            return;
        }
        let mut events = std::mem::take(&mut self.events);
        events.clear();
        let processed = session.engine.process(&mut events).is_ok();
        let mut notable = false;
        for event in events.drain(..).filter(|_| processed) {
            notable |= self.event(event);
        }
        self.events = events;
        if index == 1 {
            self.live_frames(end);
        }
        self.refresh(notable);
    }
    fn measure(&mut self, input: &BlockIn<'_>, index: usize, end: u64, arrival: Option<f64>) {
        let frames = end - input.first_frame;
        let gap = self.expected[index]
            .filter(|frame| input.first_frame > *frame)
            .map_or(0, |frame| (input.first_frame - frame).div_ceil(frames));
        self.expected[index] = Some(self.expected[index].unwrap_or(0).max(end));
        self.counts[index] += 1;
        let elapsed = self.window[index].elapsed().as_secs_f64();
        let rate = (elapsed >= 1.0).then(|| self.counts[index] as f64 / elapsed);
        if rate.is_some() {
            self.counts[index] = 0;
            self.window[index] = Instant::now();
        }
        let mut levels = self.lock_levels();
        let level = if input.stream == Stream::Reference {
            &mut levels.reference
        } else {
            &mut levels.microphone
        };
        let power = input
            .samples
            .iter()
            .map(|&value| {
                let sample = value as f64 / 32768.0;
                sample * sample
            })
            .sum::<f64>()
            / input.samples.len().max(1) as f64;
        level.filtered_dbfs = (10.0 * power.max(1e-12).log10()) as f32;
        level.pre_dbfs = input.pre_rms_dbfs.clamp(-120.0, 0.0);
        level.peak = input.pre_peak.clamp(0.0, 1.0);
        level.gaps += gap;
        level.latest_frame = Some(end - 1);
        level.blocks += 1;
        if let Some(rate) = rate {
            level.blocks_per_second = rate;
        }
        levels.arrival_ms = arrival.filter(|value| value.is_finite() && *value >= 0.0);
    }
    /// Handles one engine event; returns whether the engine view should be announced now.
    fn event(&mut self, event: EngineEvent) -> bool {
        let Some(session) = &mut self.session else {
            return false;
        };
        match event {
            EngineEvent::State(state) => {
                session.state = state;
                true
            }
            EngineEvent::Hand {
                present,
                distance_cm,
            } => {
                let changed = present != session.hand.present;
                session.hand = HandView {
                    present,
                    distance_cm: distance_cm.filter(|_| present),
                };
                changed
            }
            EngineEvent::Warning(warning) => {
                session.warning = Some((
                    warning,
                    session.frame + WARNING_SECONDS * session.rate as u64,
                ));
                true
            }
            EngineEvent::Meter(reading) => {
                let next = MeterView::from(&reading);
                let changed = session
                    .meter
                    .is_none_or(|meter| meter.label != next.label || meter.tip != next.tip);
                session.meter = Some(next);
                session.meter_pending = true;
                let due = !session.pc
                    && session
                        .phone_meter_at
                        .is_none_or(|at| at.elapsed() >= PHONE_METER_INTERVAL);
                if due {
                    session.phone_meter_at = Some(Instant::now());
                    self.send_phone(Control::Meter {
                        score: reading.score,
                        label: name(reading.label),
                        tip: name(reading.tip),
                    });
                }
                changed
            }
            EngineEvent::Poke(poke) => {
                let now = Instant::now();
                let pc = session.pc;
                if let Some((first, captured)) = session.anchor {
                    let since = now.saturating_duration_since(captured).as_secs_f64() * 1000.0;
                    let ahead =
                        (first as f64 - poke.turn_frame as f64) / session.rate as f64 * 1000.0;
                    let latency = since + ahead;
                    if (0.0..5000.0).contains(&latency) {
                        if self.latency.len() == LATENCY_POKES {
                            self.latency.pop_front();
                        }
                        self.latency.push_back(latency);
                        self.lock_levels().latency = latency_view(&self.latency);
                    }
                }
                let testing = {
                    let mut test = self.test.lock().unwrap_or_else(|error| error.into_inner());
                    if let Some(test) = test.as_mut() {
                        if test.seen < POKE_TEST_POKES && test.started.elapsed() <= POKE_TEST_TIME {
                            test.seen += 1;
                        }
                    }
                    test.is_some()
                };
                // A poke is a test when actions are off or a poke test runs; with the action
                // "Nothing" it is neither acted on nor a test.
                let acted = !testing && self.actions.as_ref().is_some_and(|gate| gate.poke());
                let test_only = testing
                    || self
                        .actions
                        .as_ref()
                        .is_none_or(|gate| !gate.enabled() || gate.testing());
                tracing::info!(
                    "Poke at {:.0} cm, depth {:.1} cm, acted: {acted}.",
                    poke.distance_cm,
                    poke.depth_cm
                );
                if let Some(observer) = &self.observer {
                    observer(Notice::Gesture(GestureEvent {
                        kind: "poke".into(),
                        at: now.duration_since(self.origin).as_secs_f64() * 1000.0,
                        distance_cm: poke.distance_cm,
                        depth_cm: poke.depth_cm,
                        confidence: poke.confidence,
                        acted,
                        test_only,
                    }));
                }
                if !pc {
                    self.send_phone(Control::Gesture {
                        kind: "poke".into(),
                        acted,
                    });
                }
                false
            }
            _ => false,
        }
    }
    fn send_phone(&self, message: Control) {
        if let Some(phone) = &self.phone {
            // The meter and gestures are momentary; a busy connection skips one.
            let _ = phone.try_send(message);
        }
    }
    /// Sends the live frame due by this microphone frame, about 30 per second of audio. Only
    /// here is the engine's snapshot taken, because it hands over the motion row.
    fn live_frames(&mut self, end: u64) {
        let Some(session) = &mut self.session else {
            return;
        };
        let (first, mut sent) = *session.live.get_or_insert((end, 0));
        let rate = session.rate as u64;
        let due = |count: u64| first + count * rate / LIVE_PER_SECOND;
        if end < due(sent + 1) {
            return;
        }
        // After a stall, skip ahead instead of sending a burst of stale frames.
        while end >= due(sent + 2) {
            sent += 1;
        }
        session.live = Some((first, sent + 1));
        let snapshot = session.engine.snapshot();
        session.state = snapshot.state;
        session.suspended = suspended_reason(&snapshot);
        if session
            .warning
            .is_some_and(|(_, until)| session.frame >= until)
        {
            session.warning = None;
        }
        let frame = LiveFrame {
            t: self.origin.elapsed().as_secs_f64() * 1000.0,
            state: snapshot.state,
            motion: snapshot.motion.to_vec(),
            hand: session.hand,
            meter: session.meter.filter(|_| session.meter_pending),
        };
        session.meter_pending = false;
        let mut live = self.live.lock().unwrap_or_else(|error| error.into_inner());
        if live.as_mut().is_some_and(|sink| !sink(&frame)) {
            live.take();
        }
    }
    fn view(&self) -> EngineView {
        self.session
            .as_ref()
            .map_or_else(EngineView::default, |session| EngineView {
                state: session.state,
                suspended: session.suspended.clone(),
                warning: session.warning.map(|(warning, _)| name(warning)),
                hand: session.hand,
                meter: session.meter,
            })
    }
    /// Updates the shared engine view and announces it when something the interface shows as
    /// text changed; the hand's distance and meter values travel in live frames.
    fn refresh(&mut self, notable: bool) {
        if self.session.is_none() {
            return;
        }
        let view = self.view();
        let previous = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let notable = notable
            || previous.state != view.state
            || previous.suspended != view.suspended
            || previous.warning != view.warning
            || previous.hand.present != view.hand.present;
        self.publish(view, notable);
    }
    fn publish(&self, view: EngineView, notify: bool) {
        *self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = view.clone();
        if notify {
            if let Some(observer) = &self.observer {
                observer(Notice::Engine(view));
            }
        }
    }
}
fn suspended_reason(snapshot: &EngineSnapshot) -> Option<String> {
    let source = matches!(
        snapshot.meter.tip,
        Tip::CaptureOff
            | Tip::NotSpeaker
            | Tip::PcHeadphones
            | Tip::PcNoMic
            | Tip::MicSilenced
            | Tip::CaptureBlocked
    );
    snapshot.suspended.then(|| {
        if snapshot.paused {
            "paused"
        } else if source {
            "source"
        } else if snapshot.state != EngineState::Running {
            "starting"
        } else if snapshot.meter.score < 15 || snapshot.meter.tip == Tip::NothingPlaying {
            "sound"
        } else {
            "settling"
        }
        .into()
    })
}
fn latency_view(values: &VecDeque<f64>) -> LatencyView {
    let mut sorted: Vec<f64> = values.iter().copied().collect();
    sorted.sort_by(f64::total_cmp);
    let at = |q: f64| {
        (!sorted.is_empty()).then(|| sorted[((sorted.len() - 1) as f64 * q).round() as usize])
    };
    LatencyView {
        pokes: sorted.len() as u32,
        median_ms: at(0.5),
        p95_ms: at(0.95),
    }
}
/// The snake_case name a unit enum serializes to.
fn name(value: impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}
fn source_status(capture: &Capture, volume: Option<f32>) -> SourceStatus {
    SourceStatus {
        capture_running: capture.state == "running",
        source: match capture.source.as_str() {
            "player" => SoundSource::Player,
            "test_sound" => SoundSource::TestSound,
            "pc" => SoundSource::Pc,
            _ => SoundSource::Capture,
        },
        // A phone reports `unknown` when it lists no output device; nothing suggests another one.
        route: match capture.route.as_str() {
            "external" => SoundRoute::External,
            "headphones" => SoundRoute::Headphones,
            "bluetooth" => SoundRoute::Bluetooth,
            _ => SoundRoute::Speaker,
        },
        mic_silenced: capture.reason.as_deref() == Some("mic_silenced"),
        mic_available: capture.reason.as_deref() != Some("mic_error"),
        media_volume: volume,
    }
}
fn input(block: &AudioBlock) -> BlockIn<'_> {
    BlockIn {
        stream: if block.stream == 0 {
            Stream::Reference
        } else {
            Stream::Microphone
        },
        first_frame: block.first_frame,
        capture_ns: if block.capture_ns == i64::MIN {
            None
        } else {
            Some(block.capture_ns)
        },
        flags: BlockFlags(block.flags),
        pre_rms_dbfs: block.pre_rms_dbfs,
        pre_peak: block.pre_peak,
        samples: &block.samples,
    }
}

#[cfg(test)]
mod tests;
