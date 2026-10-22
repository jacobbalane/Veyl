use veyl_link::{
    audio::{AudioBlock, Ping, Pong},
    crypto::{SealedChannel, Secret},
    messages::Control,
    *,
};

fn channels() -> (SealedChannel, SealedChannel) {
    (
        SealedChannel::new(Secret::new([1; 32]), Secret::new([2; 32])),
        SealedChannel::new(Secret::new([2; 32]), Secret::new([1; 32])),
    )
}
#[test]
fn duplex_keys_counters_and_json_round_trip() {
    let (mut pc, mut phone) = channels();
    for _ in 0..3 {
        let bytes = Control::RequestStart.encode().unwrap();
        let frame = pc.seal(CONTROL, &bytes).unwrap();
        let (kind, decoded) = phone.open(&frame).unwrap();
        assert_eq!(kind, CONTROL);
        assert!(matches!(
            Control::decode(&decoded).unwrap(),
            Control::RequestStart
        ));
        let pong = phone
            .seal(
                PONG,
                &Pong {
                    pc_ns: 7,
                    phone_ns: -12,
                }
                .encode(),
            )
            .unwrap();
        assert_eq!(
            Pong::decode(&pc.open(&pong).unwrap().1).unwrap().phone_ns,
            -12
        );
    }
    assert_eq!(pc.counters(), (3, 3));
    assert_eq!(phone.counters(), (3, 3));
}
#[test]
fn audio_and_ping_keep_their_exact_little_endian_layout() {
    let audio = AudioBlock {
        stream: 1,
        channels: 2,
        sample_rate: 48000,
        seq: 9,
        first_frame: 480,
        capture_ns: i64::MIN,
        frames: 2,
        flags: 5,
        pre_rms_dbfs: -20.0,
        pre_peak: 0.5,
        samples: vec![-32768, -1, 0, 32767],
    };
    let encoded = audio.encode().unwrap();
    assert_eq!(encoded.len(), 48);
    assert_eq!(&encoded[..8], &[AUDIO, 1, 2, 1, 0x80, 0xbb, 0, 0]);
    assert_eq!(&encoded[40..], &[0, 128, 255, 255, 0, 0, 255, 127]);
    assert!(AudioBlock::decode(&encoded).unwrap() == audio);
    assert_eq!(Ping(0x0102030405060708).encode(), [8, 7, 6, 5, 4, 3, 2, 1]);
    assert!(AudioBlock::decode(&encoded[..47]).is_err());
}
#[test]
fn authentication_and_replay_fail_closed() {
    let (mut pc, mut phone) = channels();
    let frame = pc.seal(PING, &Ping(0).encode()).unwrap();
    phone.open(&frame).unwrap();
    assert!(phone.open(&frame).is_err());
    assert!(phone.is_closed());
    let (mut pc, mut phone) = channels();
    let mut frame = pc.seal(PING, &Ping(0).encode()).unwrap();
    *frame.body.last_mut().unwrap() ^= 1;
    assert!(phone.open(&frame).is_err());
    assert!(phone.is_closed());
    assert!(phone
        .seal(
            PONG,
            &Pong {
                pc_ns: 0,
                phone_ns: 0
            }
            .encode()
        )
        .is_err());
}
