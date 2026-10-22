//! One task owns a connection, its counters and cancellation-safe framing.
use crate::{
    address,
    source::{PhoneSource, SoundSource},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use std::{
    collections::VecDeque,
    io,
    net::SocketAddrV4,
    time::{Duration, Instant},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, watch},
    task::JoinHandle,
    time::{interval, timeout},
};
use veyl_link::{
    audio::{AudioBlock, Ping, Pong},
    crypto::{Ephemeral, SealedChannel},
    flow::{Heartbeat, Liveness},
    messages::{AudioConfig, Bytes, Control, Handshake, Pc, Ready, Want},
    pairing::{Identity, PairedPeer},
    session::ClockSync,
    transport::{self, Connection},
    Rejection, AUDIO, CONTROL, PING, PONG,
};

pub(super) enum Mode {
    Pair,
    Session(PairedPeer),
    Probe,
}
pub(super) struct Plan {
    pub generation: u64,
    pub address: String,
    pub identity: Identity,
    pub mode: Mode,
}
pub(super) enum Event {
    Stage(u64, &'static str),
    Compare(u64, String, String),
    Paired(u64, PairedPeer, SocketAddrV4),
    Ready(u64, Ready, SocketAddrV4),
    Info(u64, Control),
    Latency(u64, f64),
    Probed(u64, String, SocketAddrV4),
    Ended(u64, &'static str),
}
pub(super) struct Worker {
    pub confirmations: mpsc::Sender<bool>,
    pub controls: mpsc::Sender<Control>,
    cancel: watch::Sender<Option<&'static str>>,
    task: JoinHandle<()>,
}
impl Worker {
    pub fn start(
        plan: Plan,
        events: mpsc::Sender<Event>,
        source: PhoneSource,
        origin: Instant,
    ) -> Self {
        let (confirmations, mut confirm_rx) = mpsc::channel(4);
        let (controls, mut control_rx) = mpsc::channel(16);
        let (cancel, mut cancel_rx) = watch::channel(None);
        let failsafe = confirmations.clone();
        let task = tokio::spawn(async move {
            let generation = plan.generation;
            let result = run(
                plan,
                &events,
                &source,
                &mut confirm_rx,
                &failsafe,
                &mut control_rx,
                &mut cancel_rx,
                origin,
            )
            .await;
            let _ = source.disconnected(generation).await;
            let reason = result.err().as_ref().map(error_code).unwrap_or("closed");
            let _ = events.send(Event::Ended(generation, reason)).await;
        });
        Self {
            confirmations,
            controls,
            cancel,
            task,
        }
    }
    pub async fn stop(mut self, reason: &'static str) {
        let _ = self.cancel.send(Some(reason));
        if timeout(Duration::from_millis(500), &mut self.task)
            .await
            .is_err()
        {
            self.task.abort();
            let _ = self.task.await;
        }
    }
}
#[allow(clippy::too_many_arguments)]
async fn run(
    plan: Plan,
    events: &mpsc::Sender<Event>,
    source: &PhoneSource,
    confirmations: &mut mpsc::Receiver<bool>,
    failsafe: &mpsc::Sender<bool>,
    controls: &mut mpsc::Receiver<Control>,
    cancel: &mut watch::Receiver<Option<&'static str>>,
    origin: Instant,
) -> io::Result<()> {
    let generation = plan.generation;
    let (mut connection, address) = tokio::select! { result=address::connect(&plan.address)=>result?, _=cancel.changed()=>return Ok(()) };
    let _ = events.send(Event::Stage(generation, "handshaking")).await;
    match plan.mode {
        Mode::Pair => {
            let progress = |code: &str, peer: &veyl_link::pairing::Peer| {
                if events
                    .try_send(Event::Compare(generation, peer.name.clone(), code.into()))
                    .is_err()
                {
                    let _ = failsafe.try_send(false);
                }
            };
            let result = {
                let pairing = transport::pair_with_progress(
                    &mut connection,
                    plan.identity,
                    progress,
                    confirmations,
                );
                tokio::pin!(pairing);
                tokio::select! {
                    result=&mut pairing=>result,
                    _=cancel.changed()=> {let _=failsafe.try_send(false);timeout(Duration::from_millis(250),&mut pairing).await.map_err(|_|io::Error::new(io::ErrorKind::TimedOut,"Pairing cancelled."))?},
                }
            };
            connection.close().await;
            let peer = result?;
            let _ = events.send(Event::Paired(generation, peer, address)).await;
            Ok(())
        }
        Mode::Probe => {
            let ephemeral = Ephemeral::generate()?;
            let hello = Handshake::Hello {
                v: 1,
                pc_id: Bytes(plan.identity.id),
                app: plan.identity.app,
                e_pub: Bytes(ephemeral.public),
                nonce: Bytes(ephemeral.nonce),
            }
            .frame()?;
            let result = tokio::select! {
                result=timeout(Duration::from_secs(5),async { connection.send(&hello).await?;let frame=connection.receive().await?;Handshake::parse(&frame) })=>result.map_err(|_|io::Error::new(io::ErrorKind::TimedOut,"Probe timed out."))?,
                _=cancel.changed()=> {connection.close().await;return Ok(())},
            };
            connection.close().await;
            match result? {
                Handshake::HelloReply { v: 1, phone_id, .. } => {
                    let _ = events
                        .send(Event::Probed(
                            generation,
                            URL_SAFE_NO_PAD.encode(phone_id.0),
                            address,
                        ))
                        .await;
                    Ok(())
                }
                Handshake::Error { code, .. } => Err(known_error(&code)),
                _ => Err(bad_message()),
            }
        }
        Mode::Session(paired) => {
            let ready = Control::Ready {
                ready: Ready::Pc {
                    pc: Pc {
                        pc_name: plan.identity.name.clone(),
                        app_version: env!("CARGO_PKG_VERSION").into(),
                    },
                    want: Want { meter: true },
                },
            };
            let authenticated = tokio::select! { result=transport::session(&mut connection,plan.identity,paired,ready)=>result?,_ = cancel.changed()=> {connection.close().await;return Ok(())} };
            let Ready::Phone { capture, .. } = &authenticated.ready else {
                return Err(bad_message());
            };
            source.status(generation, capture.clone()).await?;
            let _ = events
                .send(Event::Ready(generation, authenticated.ready, address))
                .await;
            live(
                generation,
                connection,
                authenticated.channel,
                events,
                source,
                controls,
                cancel,
                origin,
            )
            .await
        }
    }
}
#[allow(clippy::too_many_arguments)]
async fn live(
    generation: u64,
    mut connection: Connection<TcpStream>,
    mut channel: SealedChannel,
    events: &mpsc::Sender<Event>,
    source: &PhoneSource,
    controls: &mut mpsc::Receiver<Control>,
    cancel: &mut watch::Receiver<Option<&'static str>>,
    origin: Instant,
) -> io::Result<()> {
    let mut ticks = interval(Duration::from_millis(50));
    let mut heartbeat = Liveness::new(true, elapsed_ms(origin));
    let mut sync = ClockSync::default();
    let mut offset = None;
    let mut pings = VecDeque::new();
    let mut format: Option<AudioConfig> = None;
    let mut pending = VecDeque::new();
    let mut writing_since = None;
    loop {
        if connection.sender.is_empty() {
            writing_since = None;
            if let Some(message) = pending.pop_front() {
                let message: Control = message;
                connection
                    .sender
                    .enqueue(&channel.seal(CONTROL, &message.encode()?)?)?;
                writing_since = Some(Instant::now());
            }
        }
        tokio::select! {
            _=cancel.changed()=> {
                let reason=cancel.borrow().unwrap_or("user_quit");
                let _=timeout(Duration::from_millis(200),async {connection.sender.flush().await?;let frame=channel.seal(CONTROL,&Control::Bye {reason:reason.into()}.encode()?)?;connection.send(&frame).await}).await;
                connection.close().await;return Ok(());
            },
            frame=connection.receiver.receive()=> {
                let received=elapsed_ns(origin);let frame=frame?;
                let (kind,payload)=match channel.open(&frame) {
                    Ok(packet)=>packet,
                    Err(error)=> {if let Some(notice)=channel.take_close_frame() {let _=timeout(Duration::from_millis(100),async {connection.sender.flush().await?;connection.send(&notice).await}).await;} return Err(error)},
                };
                heartbeat.received(elapsed_ms(origin));
                match kind {
                    AUDIO=> {
                        let mut bytes=Vec::with_capacity(payload.len()+1);bytes.push(AUDIO);bytes.extend_from_slice(&payload);let block=AudioBlock::decode(&bytes)?;
                        let Some(config)=&format else {return Err(bad_message())};
                        if block.sample_rate!=config.sample_rate || block.frames!=config.frames_per_block || block.channels!=if block.stream==0 {config.ref_channels} else {config.mic_channels} {return Err(bad_message())}
                        let arrival=offset.filter(|_|block.capture_ns!=i64::MIN).map(|offset:i128|(received as i128-(block.capture_ns as i128-offset)) as f64/1e6);
                        source.audio(generation,block,arrival)?;
                    },
                    CONTROL=> {
                        let message=Control::decode(&payload)?;
                        match &message {
                            Control::AudioConfig {config}=> {source.configure(generation,config.clone()).await?;format=Some(config.clone());},
                            Control::CaptureState {capture}=> {source.status(generation,capture.clone()).await?;if capture.state!="running" {format=None;}},
                            Control::Bye {..}=>return Err(io::Error::new(io::ErrorKind::ConnectionAborted,"Phone closed the session.")),
                            Control::Error {code,..}=>return Err(known_error(code)),
                            _=>{},
                        }
                        events.try_send(Event::Info(generation,message)).map_err(|_|io::Error::new(io::ErrorKind::BrokenPipe,"Connection event queue is full."))?;
                    },
                    PONG=> {let pong=Pong::decode(&payload)?;if let Some(index)=pings.iter().position(|sent|*sent==pong.pc_ns) {pings.remove(index);if let Some(rtt)=received.checked_sub(pong.pc_ns) {offset=sync.observe(pong.pc_ns,received,pong.phone_ns).or(offset);let _=events.try_send(Event::Latency(generation,rtt as f64/1e6));}}},
                    _=>{},
                }
            },
            message=controls.recv(),if pending.len()<32=> {if let Some(message)=message {pending.push_back(message)} else {return Ok(())}},
            result=connection.sender.flush(),if !connection.sender.is_empty()=> {result?;},
            _=ticks.tick()=> {
                if writing_since.is_some_and(|start:Instant|start.elapsed()>=Duration::from_secs(5)) {return Err(io::Error::new(io::ErrorKind::TimedOut,"Writer stalled."))}
                match heartbeat.tick(elapsed_ms(origin)) {
                    Heartbeat::Close=>return Err(io::Error::new(io::ErrorKind::TimedOut,"Phone stopped responding.")),
                    Heartbeat::Ping=> {if connection.sender.is_empty() {let ns=elapsed_ns(origin);connection.sender.enqueue(&channel.seal(PING,&Ping(ns).encode())?)?;writing_since=Some(Instant::now());pings.push_back(ns);if pings.len()>16 {pings.pop_front();}}},
                    Heartbeat::Idle=>{},
                }
            },
        }
    }
}
fn elapsed_ns(origin: Instant) -> u64 {
    origin.elapsed().as_nanos().min(u64::MAX as u128) as u64
}
fn elapsed_ms(origin: Instant) -> u64 {
    origin.elapsed().as_millis().min(u64::MAX as u128) as u64
}
pub(super) fn error_code(error: &io::Error) -> &'static str {
    if let Some(rejection) = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<Rejection>())
    {
        return rejection.code();
    }
    match error.kind() {
        io::ErrorKind::TimedOut => "timeout",
        io::ErrorKind::InvalidInput => "invalid_address",
        io::ErrorKind::InvalidData => "bad_message",
        _ => "transport",
    }
}
fn known_error(code: &str) -> io::Error {
    // Reuse the Link core's normalized rejection without exposing peer-provided error text.
    veyl_link::rejected(code)
}
fn bad_message() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid phone message.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_host::EngineHost;
    use veyl_link::{
        messages::{Capture, Device},
        pairing::{Pairing, PairingRateLimit},
        session::Session,
    };
    fn identity(pc: bool) -> Identity {
        Identity {
            id: [if pc { 1 } else { 2 }; 16],
            name: if pc { "Desk" } else { "Test phone" }.into(),
            model: "Test".into(),
            app: "test".into(),
        }
    }
    fn capture() -> Capture {
        Capture {
            state: "running".into(),
            source: "test_sound".into(),
            route: "speaker".into(),
            reason: None,
        }
    }
    fn config() -> AudioConfig {
        AudioConfig {
            run_id: 7,
            source: "test_sound".into(),
            sample_rate: 48000,
            frames_per_block: 480,
            ref_channels: 2,
            mic_channels: 1,
            mic_source: "unprocessed".into(),
            effects_off: vec![],
            hp_cutoff_hz: 8000,
            mics: vec![],
            sound_check: None,
        }
    }
    #[tokio::test]
    async fn pairs_streams_both_channels_answers_heartbeat_and_closes_with_a_bye() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let (phone_code, mut codes) = mpsc::channel(1);
        let (requested, request) = tokio::sync::oneshot::channel();
        let phone = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut connection = Connection::new(stream);
            let mut pairing = Pairing::phone(
                identity(false),
                0,
                true,
                false,
                &mut PairingRateLimit::default(),
            );
            let mut confirmed = false;
            while !pairing.is_done() {
                let frame = connection.receive().await.unwrap();
                let mut replies = pairing.receive(&frame, 0).unwrap();
                if pairing.sas().is_some() && !confirmed {
                    phone_code
                        .send(pairing.sas().unwrap().to_owned())
                        .await
                        .unwrap();
                    replies.extend(pairing.confirm(true, 0).unwrap());
                    confirmed = true;
                }
                for reply in replies {
                    connection.send(&reply).await.unwrap();
                }
            }
            let paired = pairing.take_pairing();
            connection.close().await;
            let (stream, _) = listener.accept().await.unwrap();
            let mut connection = Connection::new(stream);
            let ready = Control::Ready {
                ready: Ready::Phone {
                    device: Device {
                        manufacturer: "Test".into(),
                        model: "Test".into(),
                        android_sdk: 26,
                        app_version: "test".into(),
                        phone_name: "Test phone".into(),
                    },
                    capture: capture(),
                },
            };
            let mut session = Session::phone(identity(false), paired, ready, 0, false).unwrap();
            while !session.is_done() {
                let frame = connection.receive().await.unwrap();
                for reply in session.receive(&frame, 0).unwrap() {
                    connection.send(&reply).await.unwrap();
                }
            }
            let mut channel = session.take_channel().unwrap();
            for control in [
                Control::AudioConfig { config: config() },
                Control::CaptureState { capture: capture() },
            ] {
                connection
                    .send(&channel.seal(CONTROL, &control.encode().unwrap()).unwrap())
                    .await
                    .unwrap();
            }
            for seq in 0..20 {
                for stream in 0..2 {
                    let channels = if stream == 0 { 2 } else { 1 };
                    let audio = AudioBlock {
                        stream,
                        channels,
                        sample_rate: 48000,
                        seq,
                        first_frame: seq as u64 * 480,
                        capture_ns: i64::MIN,
                        frames: 480,
                        flags: 12,
                        pre_rms_dbfs: -120.0,
                        pre_peak: 0.0,
                        samples: vec![0; 480 * channels as usize],
                    };
                    connection
                        .send(&channel.seal(AUDIO, &audio.encode().unwrap()[1..]).unwrap())
                        .await
                        .unwrap();
                }
            }
            let mut requested = Some(requested);
            loop {
                let frame = connection.receive().await.unwrap();
                let (kind, payload) = channel.open(&frame).unwrap();
                match kind {
                    PING => {
                        let ping = Ping::decode(&payload).unwrap();
                        connection
                            .send(
                                &channel
                                    .seal(
                                        PONG,
                                        &Pong {
                                            pc_ns: ping.0,
                                            phone_ns: ping.0 as i64 + 1_000_000_000,
                                        }
                                        .encode(),
                                    )
                                    .unwrap(),
                            )
                            .await
                            .unwrap();
                    }
                    CONTROL => match Control::decode(&payload).unwrap() {
                        Control::RequestStart => {
                            if let Some(done) = requested.take() {
                                let _ = done.send(());
                            }
                        }
                        Control::Bye { reason } => {
                            assert_eq!(reason, "forgotten");
                            break;
                        }
                        _ => {}
                    },
                    _ => panic!("Unexpected PC frame."),
                }
            }
        });
        let (engine, source) = EngineHost::start();
        let (events, mut receiver) = mpsc::channel(256);
        let origin = Instant::now();
        let pairing = Worker::start(
            Plan {
                generation: 1,
                address: address.clone(),
                identity: identity(true),
                mode: Mode::Pair,
            },
            events.clone(),
            source.clone(),
            origin,
        );
        let paired = timeout(Duration::from_secs(3), async {
            loop {
                match receiver.recv().await.unwrap() {
                    Event::Compare(_, _, code) => {
                        let other = codes.recv().await.unwrap();
                        assert!(code == other, "Pairing codes must match.");
                        pairing.confirmations.send(true).await.unwrap();
                    }
                    Event::Paired(_, peer, _) => break peer,
                    Event::Ended(_, _) => panic!("Pairing ended before confirmation."),
                    _ => {}
                }
            }
        })
        .await
        .unwrap();
        pairing.stop("user_quit").await;
        let session = Worker::start(
            Plan {
                generation: 2,
                address,
                identity: identity(true),
                mode: Mode::Session(paired),
            },
            events,
            source,
            origin,
        );
        timeout(Duration::from_secs(3), async {
            loop {
                if let Event::Ready(2, _, _) = receiver.recv().await.unwrap() {
                    break;
                }
            }
        })
        .await
        .unwrap();
        session.controls.send(Control::RequestStart).await.unwrap();
        timeout(Duration::from_secs(2), request)
            .await
            .unwrap()
            .unwrap();
        timeout(Duration::from_secs(3), async {
            loop {
                if let Event::Latency(2, rtt) = receiver.recv().await.unwrap() {
                    assert!(rtt >= 0.0);
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(engine.levels().reference.blocks, 20);
        assert_eq!(engine.levels().microphone.blocks, 20);
        assert_eq!(engine.levels().microphone.gaps, 0);
        session.stop("forgotten").await;
        timeout(Duration::from_secs(2), phone)
            .await
            .unwrap()
            .unwrap();
    }
    #[tokio::test]
    async fn manual_address_probe_preserves_not_paired_for_the_pair_offer() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let phone = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut connection = Connection::new(stream);
            let frame = connection.receive().await.unwrap();
            assert!(matches!(
                Handshake::parse(&frame).unwrap(),
                Handshake::Hello { .. }
            ));
            connection
                .send(&Handshake::error("not_paired"))
                .await
                .unwrap();
        });
        let (_engine, source) = EngineHost::start();
        let (events, mut receiver) = mpsc::channel(16);
        let worker = Worker::start(
            Plan {
                generation: 1,
                address,
                identity: identity(true),
                mode: Mode::Probe,
            },
            events,
            source,
            Instant::now(),
        );
        timeout(Duration::from_secs(2), async {
            loop {
                if let Event::Ended(_, reason) = receiver.recv().await.unwrap() {
                    assert_eq!(reason, "not_paired");
                    break;
                }
            }
        })
        .await
        .unwrap();
        worker.stop("user_quit").await;
        phone.await.unwrap();
    }
}
