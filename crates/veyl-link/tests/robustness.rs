use proptest::prelude::*;
use veyl_link::{
    audio::{AudioBlock, Ping},
    crypto::{Ephemeral, SealedChannel, Secret},
    flow::{Heartbeat, Liveness, Outbound, Reconnect, WriterQueue},
    messages::{Control, Handshake},
    *,
};
fn channel() -> SealedChannel {
    SealedChannel::new(Secret::new([3; 32]), Secret::new([3; 32]))
}
fn audio(stream: u8, seq: u32) -> AudioBlock {
    AudioBlock {
        stream,
        channels: 1,
        sample_rate: 44100,
        seq,
        first_frame: seq as u64 * 441,
        capture_ns: i64::MIN,
        frames: 1,
        flags: 0,
        pre_rms_dbfs: -120.0,
        pre_peak: 0.0,
        samples: vec![0],
    }
}

#[test]
fn limits_apply_at_the_boundary_and_unknown_types_advance_counters() {
    let mut sender = channel();
    let mut receiver = channel();
    let mut control = Control::RequestStart.encode().unwrap();
    control.resize(MAX_CONTROL, b' ');
    receiver
        .open(&sender.seal(CONTROL, &control).unwrap())
        .unwrap();
    let large = vec![0; MAX_FRAME - 26];
    let frame = sender.seal(0xee, &large).unwrap();
    assert_eq!(frame.body.len() + 1, MAX_FRAME);
    receiver
        .open(&read_frame(frame.wire().unwrap().as_slice()).unwrap())
        .unwrap();
    assert_eq!(receiver.counters().1, 2);
    let mut invalid = channel();
    control.push(b' ');
    assert!(invalid.seal(CONTROL, &control).is_err());
    assert!(invalid.is_closed());
    let mut invalid = channel();
    assert!(invalid.seal(0xee, &vec![0; MAX_FRAME - 25]).is_err());
    assert!(Control::decode(br#"{"t":"future_control","new":true}"#).is_ok());
    assert!(matches!(
        Control::decode(br#"{"t":"request_start","new":true}"#).unwrap(),
        Control::RequestStart
    ));
}
#[test]
fn skipped_counters_wrong_keys_header_changes_and_handshakes_close() {
    for mutation in 0..5 {
        let mut sender = channel();
        let mut receiver = channel();
        let mut frame = sender.seal(PING, &Ping(5).encode()).unwrap();
        match mutation {
            0 => frame.body[7] = 1,
            1 => frame.body.push(0),
            2 => frame.kind = HANDSHAKE,
            3 => {
                frame.body.truncate(24);
            }
            _ => receiver = SealedChannel::new(Secret::new([4; 32]), Secret::new([4; 32])),
        }
        assert!(receiver.open(&frame).is_err());
        assert!(receiver.is_closed());
        assert!(receiver
            .open(&sender.seal(PING, &Ping(6).encode()).unwrap())
            .is_err());
    }
    assert!(Ephemeral::from_seed([1; 32], [2; 32])
        .shared(&[0; 32])
        .is_err());
}
#[test]
fn handshake_encoding_is_bounded_canonical_and_byte_counted() {
    let (_, hello) = pairing::Pairing::pc(
        pairing::Identity {
            id: [1; 16],
            name: "Desk".into(),
            model: String::new(),
            app: "test".into(),
        },
        0,
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&hello.body).unwrap();
    value["future"] = true.into();
    assert!(Handshake::parse(&Frame {
        kind: HANDSHAKE,
        body: serde_json::to_vec(&value).unwrap()
    })
    .is_ok());
    for bad in ["AQ==", "AQ", "!", "AAAAAAAAAAAAAAAAAAAAAB"] {
        value["pc_id"] = bad.into();
        assert!(Handshake::parse(&Frame {
            kind: HANDSHAKE,
            body: serde_json::to_vec(&value).unwrap()
        })
        .is_err());
    }
    let mut body = hello.body.clone();
    body.resize(MAX_HANDSHAKE, b' ');
    assert!(Handshake::parse(&Frame {
        kind: HANDSHAKE,
        body: body.clone()
    })
    .is_ok());
    body.push(b' ');
    assert!(Handshake::parse(&Frame {
        kind: HANDSHAKE,
        body
    })
    .is_err());
    let mut body = vec![0xef, 0xbb, 0xbf];
    body.extend_from_slice(&hello.body);
    assert!(Handshake::parse(&Frame {
        kind: HANDSHAKE,
        body
    })
    .is_err());
    let (_, hello) = pairing::Pairing::pc(
        pairing::Identity {
            id: [1; 16],
            name: "é".repeat(20),
            model: String::new(),
            app: "test".into(),
        },
        0,
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&hello.body).unwrap();
    value["pc_name"] = "é".repeat(21).into();
    assert!(Handshake::parse(&Frame {
        kind: HANDSHAKE,
        body: serde_json::to_vec(&value).unwrap()
    })
    .is_err());
}
#[test]
fn queues_drop_oldest_audio_per_stream_and_preserve_control_and_gap() {
    let mut queue = WriterQueue::default();
    queue
        .push_message(CONTROL, Control::RequestStart.encode().unwrap())
        .unwrap();
    for seq in 0..100 {
        assert!(!queue.push_audio(audio(0, seq)).unwrap());
        assert!(!queue.push_audio(audio(1, seq)).unwrap());
    }
    assert!(queue.push_audio(audio(0, 100)).unwrap());
    assert_eq!(queue.len(), 201);
    assert!(matches!(
        queue.pop(),
        Some(Outbound::Message { kind: CONTROL, .. })
    ));
    assert!(
        matches!(queue.pop(), Some(Outbound::Audio(block)) if block.stream == 1 && block.seq == 0 && block.flags == 0)
    );
    assert!(
        matches!(queue.pop(), Some(Outbound::Audio(block)) if block.stream == 0 && block.seq == 1 && block.flags & 1 == 1)
    );
}
#[test]
fn heartbeat_and_reconnect_use_monotonic_deadlines() {
    let mut pc = Liveness::new(true, 0);
    assert_eq!(pc.tick(999), Heartbeat::Idle);
    assert_eq!(pc.tick(1000), Heartbeat::Ping);
    pc.received(2999);
    assert_eq!(pc.tick(5998), Heartbeat::Ping);
    assert_eq!(pc.tick(5999), Heartbeat::Close);
    let mut phone = Liveness::new(false, 0);
    assert_eq!(phone.tick(9999), Heartbeat::Idle);
    assert_eq!(phone.tick(10_000), Heartbeat::Close);
    let mut retry = Reconnect::default();
    for delay in [500, 1000, 2000, 4000, 5000, 5000] {
        assert_eq!(retry.failed(0), delay);
    }
    assert!(!retry.unreachable(29_999));
    assert!(retry.unreachable(30_000));
    retry.connected();
    assert!(!retry.unreachable(30_001));
    assert_eq!(retry.failed(30_001), 500);
}

#[test]
fn clock_sync_keeps_recent_low_rtt_offsets_and_ignores_delayed_samples() {
    let mut clock = session::ClockSync::default();
    assert_eq!(clock.observe(100, 200, 500), None);
    assert_eq!(clock.observe(300, 350, 800), Some(475));
    assert_eq!(clock.observe(400, 1400, 1_000_000), Some(475));
    assert_eq!(clock.observe(500, 540, 900), Some(475));
    assert_eq!(clock.observe(600, 640, 960), Some(380));
    assert_eq!(clock.observe(700, 699, 1), None);
}

#[test]
fn authenticated_malformed_audio_yields_one_final_error_and_closes() {
    use aes_gcm::{
        aead::{Aead, Payload},
        Aes256Gcm, KeyInit, Nonce,
    };
    let mut plain = audio(0, 0).encode().unwrap();
    plain[2] = 0;
    let length = 1 + 8 + plain.len() + 16;
    let mut aad = (length as u32).to_be_bytes().to_vec();
    aad.push(SEALED);
    aad.extend_from_slice(&0u64.to_be_bytes());
    let cipher = Aes256Gcm::new((&[3u8; 32]).into());
    let encrypted = cipher
        .encrypt(
            &Nonce::from([0u8; 12]),
            Payload {
                msg: &plain,
                aad: &aad,
            },
        )
        .unwrap();
    let mut body = 0u64.to_be_bytes().to_vec();
    body.extend_from_slice(&encrypted);
    let mut receiver = channel();
    let mut peer = channel();
    assert!(receiver.open(&Frame { kind: SEALED, body }).is_err());
    assert!(receiver.is_closed());
    let final_frame = receiver.take_close_frame().unwrap();
    assert!(receiver.take_close_frame().is_none());
    let (kind, payload) = peer.open(&final_frame).unwrap();
    assert_eq!(kind, CONTROL);
    assert!(
        matches!(Control::decode(&payload).unwrap(), Control::Error { code, .. } if code == "bad_message")
    );
}
proptest! {
    #[test]
    fn arbitrary_frames_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..2048)) { let _ = read_frame(bytes.as_slice()); }
    #[test]
    fn opaque_sealed_payloads_round_trip(payload in prop::collection::vec(any::<u8>(), 0..4096)) {
        let mut sender = channel(); let mut receiver = channel();
        let frame = sender.seal(0xf0, &payload).unwrap(); let decoded = receiver.open(&frame).unwrap();
        prop_assert!(decoded.0 == 0xf0 && decoded.1 == payload);
    }
    #[test]
    fn audio_layout_round_trips(samples in prop::collection::vec(any::<i16>(), 1..128), stereo in any::<bool>()) {
        let mut block = audio(1, 0); block.frames = samples.len() as u16; block.samples = samples;
        if stereo { block.channels = 2; block.samples = block.samples.iter().flat_map(|sample| [*sample, *sample]).collect(); }
        prop_assert!(AudioBlock::decode(&block.encode().unwrap()).unwrap() == block);
    }
}
#[tokio::test(start_paused = true)]
async fn stalled_session_writer_times_out_and_connection_cannot_be_reused() {
    let (stream, _peer) = tokio::io::duplex(1);
    let mut connection = transport::Connection::new(stream);
    let identity = pairing::Identity {
        id: [1; 16],
        name: "Desk".into(),
        model: String::new(),
        app: "test".into(),
    };
    let paired = pairing::PairedPeer {
        peer: pairing::Peer {
            id: [2; 16],
            name: "Phone".into(),
            model: String::new(),
        },
        link: Secret::new([1; 32]),
    };
    let ready = Control::Ready {
        ready: messages::Ready::Pc {
            pc: messages::Pc {
                pc_name: "Desk".into(),
                app_version: "0.0.1".into(),
            },
            want: messages::Want { meter: true },
        },
    };
    assert_eq!(
        transport::session(&mut connection, identity, paired, ready)
            .await
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::TimedOut
    );
    assert!(connection
        .sender
        .enqueue(&Handshake::error("busy"))
        .is_err());
}
