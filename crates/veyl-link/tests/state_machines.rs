use veyl_link::{
    audio::{Ping, Pong},
    crypto::Secret,
    messages::{Capture, Control, Device, Handshake, Pc, Ready, Want},
    pairing::{Identity, PairedPeer, Pairing, PairingRateLimit, Peer},
    session::Session,
    transport::{self, Connection},
    *,
};

fn identity(pc: bool) -> Identity {
    Identity {
        id: [if pc { 1 } else { 2 }; 16],
        name: if pc { "Desk" } else { "Phone" }.into(),
        model: "Test model".into(),
        app: "veyl-test/0.0.1".into(),
    }
}
fn paired(pc: bool, key: u8) -> PairedPeer {
    let id = identity(!pc);
    PairedPeer {
        peer: Peer {
            id: id.id,
            name: id.name,
            model: id.model,
        },
        link: Secret::new([key; 32]),
    }
}
fn ready(pc: bool) -> Control {
    Control::Ready {
        ready: if pc {
            Ready::Pc {
                pc: Pc {
                    pc_name: "Desk".into(),
                    app_version: "0.0.1".into(),
                },
                want: Want { meter: true },
            }
        } else {
            Ready::Phone {
                device: Device {
                    manufacturer: "Test".into(),
                    model: "Test".into(),
                    android_sdk: 26,
                    app_version: "0.0.1".into(),
                    phone_name: "Phone".into(),
                },
                capture: Capture {
                    state: "idle".into(),
                    source: "capture".into(),
                    route: "speaker".into(),
                    reason: None,
                },
            }
        },
    }
}
fn pairing() -> (Pairing, Pairing) {
    let (mut pc, hello) = Pairing::pc(identity(true), 0).unwrap();
    let mut phone = Pairing::phone(
        identity(false),
        0,
        true,
        false,
        &mut PairingRateLimit::default(),
    );
    let reply = phone.receive(&hello, 1).unwrap().remove(0);
    let reveal = pc.receive(&reply, 2).unwrap().remove(0);
    phone.receive(&reveal, 3).unwrap();
    assert!(pc.sas().is_some() && pc.sas() == phone.sas());
    (pc, phone)
}
#[test]
fn neither_side_stores_without_both_confirmations() {
    let (mut pc, mut phone) = pairing();
    let pc_confirmation = pc.confirm(true, 4).unwrap().remove(0);
    phone.receive(&pc_confirmation, 5).unwrap();
    assert!(pc.take_pairing().is_none() && phone.take_pairing().is_none());
    let phone_confirmation = phone.confirm(true, 6).unwrap().remove(0);
    pc.receive(&phone_confirmation, 7).unwrap();
    let pc_pair = pc.take_pairing().unwrap();
    let phone_pair = phone.take_pairing().unwrap();
    assert!(veyl_link::crypto::matches(
        pc_pair.link.expose(),
        phone_pair.link.expose()
    ));
    assert!(pc.sas().is_none() && phone.sas().is_none());
}
#[test]
fn pairing_rejects_commit_tampering_bad_mac_cancel_and_deadlines() {
    let (mut pc, hello) = Pairing::pc(identity(true), 0).unwrap();
    let mut phone = Pairing::phone(
        identity(false),
        0,
        true,
        false,
        &mut PairingRateLimit::default(),
    );
    let reply = phone.receive(&hello, 0).unwrap().remove(0);
    let reveal = pc.receive(&reply, 0).unwrap().remove(0);
    let mut value: serde_json::Value = serde_json::from_slice(&reveal.body).unwrap();
    value["nonce"] = serde_json::to_value(messages::Bytes([9; 32])).unwrap();
    let changed = Frame {
        kind: HANDSHAKE,
        body: serde_json::to_vec(&value).unwrap(),
    };
    assert_eq!(phone.receive(&changed, 1).unwrap().len(), 1);
    assert!(phone.failure() == Some("mismatch"));
    let (_, mut phone) = pairing();
    phone
        .receive(
            &Handshake::PairConfirm {
                mac: messages::Bytes([0; 32]),
            }
            .frame()
            .unwrap(),
            5,
        )
        .unwrap();
    assert!(phone.failure() == Some("bad_mac"));
    assert!(phone.take_pairing().is_none());
    let (mut pc, _) = pairing();
    pc.confirm(false, 5).unwrap();
    assert!(pc.failure() == Some("user_cancelled"));
    let (mut pc, _) = pairing();
    pc.tick(120_002).unwrap();
    assert!(pc.failure() == Some("timeout"));
    let (mut pc, _) = Pairing::pc(identity(true), 0).unwrap();
    pc.tick(150_000).unwrap();
    assert!(pc.failure() == Some("timeout"));
}
#[test]
fn phone_gates_and_rate_limit_survive_connections() {
    let (_, hello) = Pairing::pc(identity(true), 0).unwrap();
    let mut limit = PairingRateLimit::default();
    for (foreground, busy, reason) in [(false, false, "not_in_foreground"), (true, true, "busy")] {
        let mut phone = Pairing::phone(identity(false), 0, foreground, busy, &mut limit);
        let reply = phone.receive(&hello, 0).unwrap();
        assert!(
            matches!(Handshake::parse(&reply[0]).unwrap(), Handshake::PairAbort { reason: actual } if actual == reason)
        );
    }
    assert!(limit.admit(0));
    assert!(!limit.admit(2999));
    assert!(limit.admit(3000));
    for attempt in 2..10 {
        assert!(limit.admit(attempt * 3000));
    }
    assert!(!limit.admit(30_000));
    assert!(!limit.admit(629_999));
    assert!(limit.admit(630_000));
}
#[test]
fn sessions_require_ready_identity_correct_key_and_five_second_deadline() {
    let (mut pc, hello) = Session::pc(identity(true), paired(true, 7), ready(true), 0).unwrap();
    let mut phone = Session::phone(
        identity(false),
        Some(paired(false, 7)),
        ready(false),
        0,
        false,
    )
    .unwrap();
    let frames = phone.receive(&hello, 1).unwrap();
    pc.receive(&frames[0], 2).unwrap();
    assert!(pc.take_channel().is_none() && phone.take_channel().is_none());
    let answer = pc.receive(&frames[1], 3).unwrap();
    phone.receive(&answer[0], 4).unwrap();
    assert_eq!(pc.take_channel().unwrap().counters(), (1, 1));
    assert_eq!(phone.take_channel().unwrap().counters(), (1, 1));
    let (mut pc, hello) = Session::pc(identity(true), paired(true, 8), ready(true), 0).unwrap();
    let mut phone = Session::phone(
        identity(false),
        Some(paired(false, 7)),
        ready(false),
        0,
        false,
    )
    .unwrap();
    let frames = phone.receive(&hello, 1).unwrap();
    pc.receive(&frames[0], 2).unwrap();
    assert!(pc.receive(&frames[1], 3).is_err());
    assert!(pc.is_failed());
    let (mut pc, _) = Session::pc(identity(true), paired(true, 7), ready(true), 0).unwrap();
    assert!(pc.tick(5000).is_err());
    let (_, hello) = Session::pc(identity(true), paired(true, 7), ready(true), 0).unwrap();
    let mut unknown = Session::phone(identity(false), None, ready(false), 0, false).unwrap();
    assert!(
        matches!(Handshake::parse(&unknown.receive(&hello, 0).unwrap()[0]).unwrap(), Handshake::Error { code, .. } if code == "not_paired")
    );
    let mut wrong_id = paired(true, 7);
    wrong_id.peer.id = [99; 16];
    let (mut pc, hello) = Session::pc(identity(true), wrong_id, ready(true), 0).unwrap();
    let mut phone = Session::phone(
        identity(false),
        Some(paired(false, 7)),
        ready(false),
        0,
        false,
    )
    .unwrap();
    let frames = phone.receive(&hello, 1).unwrap();
    assert!(pc.receive(&frames[0], 2).is_err());
}

#[tokio::test]
async fn async_pc_pairs_and_authenticates_over_fragmented_in_memory_duplex() {
    let (pc_stream, phone_stream) = tokio::io::duplex(19);
    let (confirm_tx, mut confirm_rx) = tokio::sync::mpsc::channel(1);
    let pc = async {
        let mut connection = Connection::new(pc_stream);
        let paired = transport::pair(
            &mut connection,
            identity(true),
            |_| {
                confirm_tx.try_send(true).unwrap();
            },
            &mut confirm_rx,
        )
        .await
        .unwrap();
        let mut channel = transport::session(&mut connection, identity(true), paired, ready(true))
            .await
            .unwrap();
        connection
            .send(&channel.seal(PING, &Ping(42).encode()).unwrap())
            .await
            .unwrap();
        let frame = connection.receive().await.unwrap();
        let (kind, bytes) = channel.open(&frame).unwrap();
        assert_eq!(kind, PONG);
        assert_eq!(
            Pong::decode(&bytes).unwrap(),
            Pong {
                pc_ns: 42,
                phone_ns: -13
            }
        );
    };
    let phone = async {
        let mut connection = Connection::new(phone_stream);
        let mut machine = Pairing::phone(
            identity(false),
            0,
            true,
            false,
            &mut PairingRateLimit::default(),
        );
        let mut confirmed = false;
        while !machine.is_done() {
            let frame = connection.receive().await.unwrap();
            let mut outbound = machine.receive(&frame, 0).unwrap();
            if machine.sas().is_some() && !confirmed {
                outbound.extend(machine.confirm(true, 0).unwrap());
                confirmed = true;
            }
            for frame in outbound {
                connection.send(&frame).await.unwrap();
            }
        }
        let mut session = Session::phone(
            identity(false),
            machine.take_pairing(),
            ready(false),
            0,
            false,
        )
        .unwrap();
        while !session.is_done() {
            let frame = connection.receive().await.unwrap();
            let outbound = session.receive(&frame, 0).unwrap();
            for frame in outbound {
                connection.send(&frame).await.unwrap();
            }
        }
        let mut channel = session.take_channel().unwrap();
        let frame = connection.receive().await.unwrap();
        let (kind, bytes) = channel.open(&frame).unwrap();
        assert_eq!(kind, PING);
        connection
            .send(
                &channel
                    .seal(
                        PONG,
                        &Pong {
                            pc_ns: Ping::decode(&bytes).unwrap().0,
                            phone_ns: -13,
                        }
                        .encode(),
                    )
                    .unwrap(),
            )
            .await
            .unwrap();
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(pc, phone);
    })
    .await
    .unwrap();
}
