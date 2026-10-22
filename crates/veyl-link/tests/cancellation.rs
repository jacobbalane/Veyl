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
