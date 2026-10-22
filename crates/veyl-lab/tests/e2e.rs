//! No audio hardware, disk recording or plaintext diagnostic output is used by this test.
use std::{io, sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::mpsc,
    time::{self, Instant},
};
use veyl_lab::{fake_phone::FakePhone, recording::Input};
use veyl_link::{
    audio::{AudioBlock, Ping, Pong},
    messages::{Control, Pc, Ready, Want},
    pairing::{Identity, PairedPeer},
    transport::{self, Connection},
    AUDIO, CONTROL, PING, PONG,
};

fn pc() -> Identity {
    Identity {
        id: [1; 16],
        name: "End-to-end test".into(),
        model: String::new(),
        app: env!("CARGO_PKG_VERSION").into(),
    }
}
fn ready() -> Control {
    Control::Ready {
        ready: Ready::Pc {
            pc: Pc {
                pc_name: pc().name,
                app_version: pc().app,
            },
            want: Want { meter: true },
        },
    }
}
async fn pair(phone: &FakePhone, mut phone_codes: mpsc::Receiver<String>) -> PairedPeer {
    let mut connection = Connection::new(TcpStream::connect(phone.address()).await.unwrap());
    let (confirm, mut decisions) = mpsc::channel(1);
    let keep_decisions_open = confirm.clone();
    let (code_tx, mut pc_codes) = mpsc::channel(1);
    let compare = tokio::spawn(async move {
        let pc_code = pc_codes.recv().await.unwrap();
        let phone_code = phone_codes.recv().await.unwrap();
        // Avoid formatting either ephemeral comparison code if the assertion fails.
        assert!(pc_code == phone_code);
        confirm.send(true).await.unwrap();
    });
    let paired = transport::pair(
        &mut connection,
        pc(),
        move |code| {
            assert!(code_tx.try_send(code.to_owned()).is_ok());
        },
        &mut decisions,
    )
    .await
    .unwrap();
    compare.await.unwrap();
    drop(keep_decisions_open);
    connection.close().await;
    while phone.report().paired != 1 {
        tokio::task::yield_now().await;
    }
    paired
}

#[tokio::test]
async fn forty_second_scenario_pairs_authenticates_and_delivers_all_eight_thousand_blocks() {
    time::timeout(Duration::from_secs(30), async {
        let input = Input::generated(veyl_sim::poke_basic(1, 40.0).unwrap());
        let expected: Vec<_> = (0..input.blocks.len())
            .map(|i| input.block(i, 0).unwrap())
            .collect();
        let (codes, phone_codes) = mpsc::channel(1);
        let phone = FakePhone::bind(
            input,
            0,
            false,
            Arc::new(move |code| {
                assert!(codes.try_send(code.to_owned()).is_ok());
            }),
        )
        .await
        .unwrap();
        let paired = pair(&phone, phone_codes).await;
        let mut connection = Connection::new(TcpStream::connect(phone.address()).await.unwrap());
        let mut authenticated = transport::session(&mut connection, pc(), paired, ready())
            .await
            .unwrap();
        assert!(matches!(authenticated.ready, Ready::Phone { .. }));
        let mut counts = [0_u64; 2];
        let (mut configured, mut running, mut stats, mut pongs) = (false, false, false, 0);
        let mut capture_start = None;
        let mut total = 0;
        let mut ended = false;
        loop {
            let frame = connection.receive().await.unwrap();
            // A counter skip, reordering or bad tag fails here before accepting a block.
            let (kind, payload) = authenticated.channel.open(&frame).unwrap();
            match kind {
                CONTROL => match Control::decode(&payload).unwrap() {
                    Control::AudioConfig { config } => {
                        assert_eq!(
                            (
                                config.sample_rate,
                                config.frames_per_block,
                                config.ref_channels,
                                config.mic_channels,
                                config.hp_cutoff_hz
                            ),
                            (48000, 480, 2, 2, 8000)
                        );
                        configured = true;
                    }
                    Control::CaptureState { capture } if capture.state == "running" => {
                        running = true
                    }
                    Control::CaptureState { capture } if capture.state == "idle" => ended = true,
                    Control::Stats { dropped_blocks, .. } => {
                        assert_eq!(dropped_blocks, 0);
                        stats = true;
                    }
                    _ => {}
                },
                AUDIO => {
                    assert!(configured && running);
                    let mut plain = vec![AUDIO];
                    plain.extend_from_slice(&payload);
                    let block = AudioBlock::decode(&plain).unwrap();
                    let expected = &expected[total];
                    assert_eq!(
                        (
                            block.stream,
                            block.seq,
                            block.first_frame,
                            block.frames,
                            block.flags
                        ),
                        (
                            expected.stream,
                            expected.seq,
                            expected.first_frame,
                            expected.frames,
                            expected.flags
                        )
                    );
                    assert!(block.samples == expected.samples);
                    assert_eq!(block.pre_rms_dbfs, expected.pre_rms_dbfs);
                    assert_eq!(block.pre_peak, expected.pre_peak);
                    let start = *capture_start.get_or_insert(block.capture_ns);
                    assert_eq!(block.capture_ns - start, expected.capture_ns);
                    counts[block.stream as usize] += 1;
                    total += 1;
                    if total % 400 == 0 {
                        connection
                            .send(
                                &authenticated
                                    .channel
                                    .seal(PING, &Ping(total as u64).encode())
                                    .unwrap(),
                            )
                            .await
                            .unwrap();
                    }
                }
                PONG => {
                    assert!(Pong::decode(&payload).unwrap().phone_ns >= 0);
                    pongs += 1;
                }
                _ => panic!("Unexpected simulator message."),
            }
            if ended && pongs == 20 {
                break;
            }
        }
        assert_eq!(counts, [4000, 4000]);
        assert!(stats && pongs == 20);
        connection
            .send(
                &authenticated
                    .channel
                    .seal(
                        CONTROL,
                        &Control::Bye {
                            reason: "user".into(),
                        }
                        .encode()
                        .unwrap(),
                    )
                    .unwrap(),
            )
            .await
            .unwrap();
        let report = phone.report();
        assert_eq!(report.blocks, counts);
        assert_eq!(report.sessions, 1);
        assert_eq!(report.decryption_errors, 0);
        assert_eq!(report.connection_errors, 0);
        phone.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn realtime_pacing_busy_rejection_tamper_and_shutdown_are_bounded() {
    time::timeout(Duration::from_secs(10), async {
        let (codes, phone_codes) = mpsc::channel(1);
        let phone = FakePhone::bind(Input::generated(veyl_sim::poke_basic(2, 0.2).unwrap()), 0, true, Arc::new(move |code| {
            assert!(codes.try_send(code.to_owned()).is_ok());
        })).await.unwrap();
        let paired = pair(&phone, phone_codes).await;
        let mut connection = Connection::new(TcpStream::connect(phone.address()).await.unwrap());
        let mut session = transport::session(&mut connection, pc(), paired, ready()).await.unwrap();
        let mut busy = Connection::new(TcpStream::connect(phone.address()).await.unwrap());
        let (_, hello) = veyl_link::pairing::Pairing::pc(pc(), 0).unwrap();
        busy.send(&hello).await.unwrap();
        assert!(matches!(veyl_link::messages::Handshake::parse(&busy.receive().await.unwrap()).unwrap(), veyl_link::messages::Handshake::PairAbort { reason } if reason == "busy"));
        busy.close().await;
        let started = Instant::now();
        let mut blocks = 0;
        while blocks < 40 {
            let (kind, _) = session.channel.open(&connection.receive().await.unwrap()).unwrap();
            if kind == AUDIO { blocks += 1; }
        }
        // The busy rejection consumed at most 50 ms while the active stream kept running.
        assert!(started.elapsed() >= Duration::from_millis(100));
        assert!(started.elapsed() < Duration::from_secs(2));
        let mut corrupt = session.channel.seal(PING, &Ping(1).encode()).unwrap();
        *corrupt.body.last_mut().unwrap() ^= 1;
        connection.send(&corrupt).await.unwrap();
        while phone.report().decryption_errors == 0 { tokio::task::yield_now().await; }
        assert_eq!(phone.report().decryption_errors, 1);
        let address = phone.address();
        phone.shutdown().await;
        assert!(matches!(TcpStream::connect(address).await.unwrap_err().kind(), io::ErrorKind::ConnectionRefused));
    }).await.unwrap();
}
