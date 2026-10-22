//! A loopback-only phone simulator using the same pairing and session core as the apps.
use crate::recording::Input;
use std::{
    collections::BTreeMap,
    future::pending,
    io,
    net::{Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch,
    task::JoinHandle,
    time::{self, Instant},
};
use veyl_link::{
    audio::{Ping, Pong},
    crypto::{SealedChannel, Secret},
    messages::{AudioConfig, Capture, Control, Device, Handshake, Ready},
    pairing::{Identity, PairedPeer, Pairing, PairingRateLimit},
    session::Session,
    transport::Connection,
    Frame, AUDIO, CONTROL, PING, PONG,
};

pub type Comparison = Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Clone, Copy, Default, Debug)]
pub struct Report {
    pub paired: usize,
    pub sessions: u64,
    pub blocks: [u64; 2],
    pub decryption_errors: u64,
    pub connection_errors: u64,
}
struct State {
    pairings: BTreeMap<[u8; 16], PairedPeer>,
    limiter: PairingRateLimit,
    report: Report,
}
struct Phone {
    identity: Identity,
    input: Input,
    realtime: bool,
    comparison: Comparison,
    origin: Instant,
    state: Mutex<State>,
}
impl Phone {
    fn ms(&self) -> u64 {
        self.origin.elapsed().as_millis().min(u64::MAX as u128) as u64
    }
    fn ns(&self) -> i64 {
        self.origin.elapsed().as_nanos().min(i64::MAX as u128) as i64
    }
}
pub struct FakePhone {
    address: SocketAddr,
    phone: Arc<Phone>,
    stop: watch::Sender<bool>,
    task: Option<JoinHandle<()>>,
}
impl FakePhone {
    pub async fn bind(
        input: Input,
        port: u16,
        realtime: bool,
        comparison: Comparison,
    ) -> io::Result<Self> {
        // Validate before opening the listener. Audio remains in memory unless the caller
        // explicitly chose a recording as input.
        for index in 0..input.blocks.len() {
            input.block(index, 0)?;
        }
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
        let address = listener.local_addr()?;
        let mut id = [0; 16];
        getrandom::fill(&mut id).map_err(|_| invalid("Random identity unavailable."))?;
        let phone = Arc::new(Phone {
            identity: Identity {
                id,
                name: "Veyl simulator".into(),
                model: "Simulator".into(),
                app: env!("CARGO_PKG_VERSION").into(),
            },
            input,
            realtime,
            comparison,
            origin: Instant::now(),
            state: Mutex::new(State {
                pairings: BTreeMap::new(),
                limiter: PairingRateLimit::default(),
                report: Report::default(),
            }),
        });
        let (stop, mut stopped) = watch::channel(false);
        let server = phone.clone();
        let task = tokio::spawn(async move {
            let mut active: Option<JoinHandle<()>> = None;
            loop {
                tokio::select! {
                    biased;
                    _ = stopped.changed() => break,
                    _ = async {
                        match active.as_mut() {
                            Some(task) => { let _ = task.await; },
                            None => pending::<()>().await,
                        }
                    } => { active = None; },
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { break; };
                        let _ = stream.set_nodelay(true);
                        if active.is_some() {
                            // Keep rejection bounded even if a local client never sends hello.
                            let _ = time::timeout(Duration::from_millis(250), reject_busy(stream)).await;
                        } else {
                            let server = server.clone();
                            active = Some(tokio::spawn(async move {
                                if serve(stream, &server).await.is_err() {
                                    server.state.lock().unwrap().report.connection_errors += 1;
                                }
                            }));
                        }
                    }
                }
            }
            if let Some(active) = active {
                active.abort();
                let _ = active.await;
            }
        });
        Ok(Self {
            address,
            phone,
            stop,
            task: Some(task),
        })
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn report(&self) -> Report {
        self.phone.state.lock().unwrap().report
    }
    pub async fn shutdown(mut self) {
        let _ = self.stop.send(true);
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}
impl Drop for FakePhone {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
    }
}
async fn reject_busy(stream: TcpStream) -> io::Result<()> {
    let mut connection = Connection::new(stream);
    let first = connection.receive().await?;
    let reply = match Handshake::parse(&first)? {
        Handshake::PairHello { .. } => Handshake::PairAbort {
            reason: "busy".into(),
        }
        .frame()?,
        _ => Handshake::error("busy"),
    };
    connection.send(&reply).await?;
    // Drain a possible in-flight frame before closing, avoiding an unread-data reset.
    let _ = time::timeout(Duration::from_millis(50), connection.receive()).await;
    connection.close().await;
    Ok(())
}
async fn serve(stream: TcpStream, phone: &Phone) -> io::Result<()> {
    let mut connection = Connection::new(stream);
    let first = time::timeout(Duration::from_secs(5), connection.receive()).await??;
    match Handshake::parse(&first)? {
        Handshake::PairHello { .. } => {
            time::timeout(
                Duration::from_secs(151),
                pair(&mut connection, first, phone),
            )
            .await?
        }
        Handshake::Hello { pc_id, .. } => {
            let paired = phone
                .state
                .lock()
                .unwrap()
                .pairings
                .get(&pc_id.0)
                .map(|paired| PairedPeer {
                    peer: paired.peer.clone(),
                    link: Secret::new(*paired.link.expose()),
                });
            let run_id = phone.state.lock().unwrap().report.sessions + 1;
            let mut machine = Session::phone(
                phone.identity.clone(),
                paired,
                ready(phone),
                phone.ms(),
                false,
            )?;
            let channel = time::timeout(Duration::from_secs(5), async {
                let mut next = first;
                loop {
                    for reply in machine.receive(&next, phone.ms())? {
                        connection.send(&reply).await?;
                    }
                    if machine.is_failed() {
                        return Err(veyl_link::rejected(
                            machine.failure().unwrap_or("bad_message"),
                        ));
                    }
                    if machine.is_done() {
                        return machine
                            .take_channel()
                            .ok_or_else(|| invalid("No session channel."));
                    }
                    next = connection.receive().await?;
                }
            })
            .await??;
            phone.state.lock().unwrap().report.sessions = run_id;
            stream_audio(&mut connection, channel, phone, run_id).await
        }
        _ => Err(invalid("Expected a phone handshake.")),
    }
}
async fn pair(
    connection: &mut Connection<TcpStream>,
    first: Frame,
    phone: &Phone,
) -> io::Result<()> {
    let mut pairing = Pairing::phone(
        phone.identity.clone(),
        phone.ms(),
        true,
        false,
        &mut phone.state.lock().unwrap().limiter,
    );
    let mut outbound = pairing.receive(&first, phone.ms())?;
    let mut confirmed = false;
    let mut ticks = time::interval(Duration::from_millis(100));
    loop {
        for frame in outbound.drain(..) {
            connection.sender.enqueue(&frame)?;
        }
        if let Some(code) = pairing.sas().filter(|_| !confirmed) {
            (phone.comparison)(code);
            for frame in pairing.confirm(true, phone.ms())? {
                connection.sender.enqueue(&frame)?;
            }
            confirmed = true;
        }
        if connection.sender.is_empty() {
            if let Some(paired) = pairing.take_pairing() {
                let mut state = phone.state.lock().unwrap();
                if state.pairings.len() >= 64 && !state.pairings.contains_key(&paired.peer.id) {
                    return Err(invalid("Simulator pairing limit reached."));
                }
                state.pairings.insert(paired.peer.id, paired);
                state.report.paired = state.pairings.len();
                return Ok(());
            }
            if let Some(reason) = pairing.failure() {
                return Err(veyl_link::rejected(reason));
            }
        }
        outbound = tokio::select! {
            frame = connection.receiver.receive(), if !pairing.is_done() && pairing.failure().is_none() => pairing.receive(&frame?, phone.ms())?,
            _ = ticks.tick() => pairing.tick(phone.ms())?,
            result = connection.sender.flush(), if !connection.sender.is_empty() => { result?; vec![] }
        };
    }
}
fn capture(running: bool) -> Capture {
    Capture {
        state: if running { "running" } else { "idle" }.into(),
        source: "test_sound".into(),
        route: "speaker".into(),
        reason: None,
    }
}
fn ready(phone: &Phone) -> Control {
    Control::Ready {
        ready: Ready::Phone {
            device: Device {
                manufacturer: "Veyl".into(),
                model: "Simulator".into(),
                android_sdk: 26,
                app_version: phone.identity.app.clone(),
                phone_name: phone.identity.name.clone(),
            },
            capture: capture(true),
        },
    }
}
fn config(phone: &Phone, run_id: u64) -> Control {
    let meta = &phone.input.meta;
    Control::AudioConfig {
        config: AudioConfig {
            run_id,
            source: "test_sound".into(),
            sample_rate: meta.sample_rate,
            frames_per_block: (meta.sample_rate / 100) as u16,
            ref_channels: meta.ref_channels,
            mic_channels: meta.mic_channels,
            mic_source: "unprocessed".into(),
            effects_off: vec![],
            hp_cutoff_hz: meta.hp_cutoff_hz,
            mics: vec![],
            sound_check: None,
        },
    }
}
fn stats() -> Control {
    Control::Stats {
        queue_ms: 0,
        dropped_blocks: 0,
        media_volume: 0.5,
        battery_pct: 100,
        charging: false,
        thermal: "none".into(),
        cpu_pct: 0.0,
    }
}
fn enqueue(
    connection: &mut Connection<TcpStream>,
    channel: &mut SealedChannel,
    control: Control,
) -> io::Result<()> {
    connection
        .sender
        .enqueue(&channel.seal(CONTROL, &control.encode()?)?)
}
async fn stream_audio(
    connection: &mut Connection<TcpStream>,
    mut channel: SealedChannel,
    phone: &Phone,
    run_id: u64,
) -> io::Result<()> {
    enqueue(connection, &mut channel, config(phone, run_id))?;
    enqueue(
        connection,
        &mut channel,
        Control::CaptureState {
            capture: capture(true),
        },
    )?;
    enqueue(connection, &mut channel, stats())?;
    time::timeout(Duration::from_secs(5), connection.sender.flush()).await??;
    let start = Instant::now();
    let start_ns = phone.ns();
    let mut index = 0;
    let mut idle_sent = false;
    let mut last_receive = Instant::now();
    let mut write_started = Instant::now();
    let mut stats_at = Instant::now() + Duration::from_secs(1);
    let mut pending_pongs = 0;
    loop {
        // At most one pending audio frame and a few controls. Seal only in wire order.
        // Independent read/write deadlines survive cancellation of a partial frame.
        let due = if phone.realtime && index < phone.input.blocks.len() {
            start + Duration::from_nanos(phone.input.due_ns(index)?)
        } else {
            Instant::now()
        };
        let empty = connection.sender.is_empty();
        if empty {
            write_started = Instant::now();
            pending_pongs = 0;
        }
        tokio::select! {
            frame = connection.receiver.receive() => {
                let (kind, payload) = channel.open(&frame?).inspect_err(|_| {
                    phone.state.lock().unwrap().report.decryption_errors += 1;
                })?;
                last_receive = Instant::now();
                match kind {
                    PING => {
                        pending_pongs += 1;
                        if pending_pongs > 256 { return Err(invalid("PC heartbeat queue exceeded.")); }
                        let ping = Ping::decode(&payload)?;
                        connection.sender.enqueue(&channel.seal(PONG, &Pong { pc_ns: ping.0, phone_ns: phone.ns() }.encode())?)?;
                    }
                    CONTROL => match Control::decode(&payload)? {
                        Control::Bye { .. } => return Ok(()),
                        Control::RequestStart | Control::Meter { .. } | Control::Gesture { .. } | Control::Unknown => {},
                        _ => return Err(invalid("Unexpected PC control.")),
                    },
                    _ => return Err(invalid("Unexpected PC message.")),
                }
                if !empty && write_started.elapsed() >= Duration::from_secs(5) { return Err(io::ErrorKind::TimedOut.into()); }
            }
            result = connection.sender.flush(), if !empty => { result?; }
            _ = async { if phone.realtime { time::sleep_until(due).await; } }, if empty && index < phone.input.blocks.len() => {
                let block = phone.input.block(index, start_ns)?;
                connection.sender.enqueue(&channel.seal(AUDIO, &block.encode()?[1..])?)?;
                phone.state.lock().unwrap().report.blocks[block.stream as usize] += 1;
                index += 1;
            }
            _ = time::sleep_until(stats_at), if empty => {
                enqueue(connection, &mut channel, stats())?;
                stats_at = Instant::now() + Duration::from_secs(1);
            }
            _ = std::future::ready(()), if empty && index == phone.input.blocks.len() && !idle_sent => {
                enqueue(connection, &mut channel, Control::CaptureState { capture: capture(false) })?;
                idle_sent = true;
            }
            _ = time::sleep_until(last_receive + Duration::from_secs(10)) => return Err(io::ErrorKind::TimedOut.into()),
            _ = time::sleep_until(write_started + Duration::from_secs(5)), if !empty => return Err(io::ErrorKind::TimedOut.into()),
        }
    }
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
