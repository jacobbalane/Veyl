use veyl_link::{
    messages::Handshake,
    pairing::{Identity, Pairing, PairingRateLimit},
    transport::{self, Connection},
};
fn identity() -> Identity {
    Identity {
        id: [1; 16],
        name: "Desk".into(),
        model: String::new(),
        app: "test".into(),
    }
}
#[test]
fn both_roles_allow_cancellation_before_a_code_exists() {
    let (mut pc, _) = Pairing::pc(identity(), 0).unwrap();
    let mut phone = Pairing::phone(identity(), 0, true, false, &mut PairingRateLimit::default());
    for machine in [&mut pc, &mut phone] {
        assert!(
            matches!(Handshake::parse(&machine.cancel().unwrap()[0]).unwrap(), Handshake::PairAbort { reason } if reason == "user_cancelled")
        );
        assert!(machine.take_pairing().is_none());
        assert!(machine.sas().is_none());
        assert!(machine.cancel().is_err());
    }
}
#[tokio::test]
async fn pc_cancel_before_reply_sends_abort_without_waiting_for_timeout() {
    let (stream, peer) = tokio::io::duplex(19);
    let mut connection = Connection::new(stream);
    let mut peer = Connection::new(peer);
    let (decisions, mut receiver) = tokio::sync::mpsc::channel(1);
    decisions.send(false).await.unwrap();
    let pc = async {
        assert!(transport::pair(
            &mut connection,
            identity(),
            |_| panic!("Code must not appear."),
            &mut receiver
        )
        .await
        .is_err());
    };
    let phone = async {
        assert!(matches!(
            Handshake::parse(&peer.receive().await.unwrap()).unwrap(),
            Handshake::PairHello { .. }
        ));
        assert!(
            matches!(Handshake::parse(&peer.receive().await.unwrap()).unwrap(), Handshake::PairAbort { reason } if reason == "user_cancelled")
        );
    };
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        tokio::join!(pc, phone);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn pc_can_cancel_after_confirming_while_the_phone_has_not_confirmed() {
    let (stream, peer) = tokio::io::duplex(64);
    let (decisions, mut receiver) = tokio::sync::mpsc::channel(1);
    let confirmations = decisions.clone();
    let pc = async {
        let mut connection = Connection::new(stream);
        assert!(transport::pair(
            &mut connection,
            identity(),
            |_| {
                confirmations.try_send(true).unwrap();
            },
            &mut receiver
        )
        .await
        .is_err());
    };
    let phone = async {
        let mut connection = Connection::new(peer);
        let mut machine =
            Pairing::phone(identity(), 0, true, false, &mut PairingRateLimit::default());
        loop {
            let frame = connection.receive().await.unwrap();
            let message = Handshake::parse(&frame).unwrap();
            let confirmed = matches!(message, Handshake::PairConfirm { .. });
            let cancelled = matches!(message, Handshake::PairAbort { .. });
            for reply in machine.receive(&frame, 0).unwrap() {
                connection.send(&reply).await.unwrap();
            }
            if confirmed {
                decisions.send(false).await.unwrap();
            }
            if cancelled {
                assert!(machine.failure() == Some("user_cancelled"));
                assert!(machine.take_pairing().is_none());
                break;
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        tokio::join!(pc, phone);
    })
    .await
    .unwrap();
}
