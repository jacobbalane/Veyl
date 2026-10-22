//! Public, fixed test inputs. Never use these private keys or nonces in a connection.
use crate::{
    audio::{AudioBlock, Ping, Pong},
    crypto::{self, Ephemeral, PairKeys, SessionKeys},
    messages::{Bytes, Handshake},
    Frame, AUDIO, CONTROL, PING, PONG,
};
use serde_json::{json, Value};

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
pub fn controls() -> Vec<String> {
    [
        r#"{"t":"ready","device":{"manufacturer":"Test","model":"Test phone","android_sdk":26,"app_version":"0.0.1","phone_name":"Phone"},"capture":{"state":"idle","source":"capture","route":"speaker","reason":null}}"#,
        r#"{"t":"ready","pc":{"pc_name":"Desk","app_version":"0.0.1"},"want":{"meter":true}}"#,
        r#"{"t":"audio_config","run_id":7,"sample_rate":48000,"frames_per_block":480,"ref_channels":2,"mic_channels":2,"mic_source":"unprocessed","effects_off":["ns","agc"],"hp_cutoff_hz":8000,"source":"capture","sound_check":{"mic_source":"unprocessed","mic_channels":2,"score":0.71,"mean_coherence":0.82},"mics":[{"index":0,"type":"builtin","position_m":[-0.02,-0.075,0.0],"channels":[0]},{"index":1,"type":"builtin","position_m":null,"channels":[1]}]}"#,
        r#"{"t":"capture_state","state":"running","source":"capture","route":"speaker","reason":null}"#,
        r#"{"t":"stats","queue_ms":12,"dropped_blocks":0,"media_volume":0.6,"battery_pct":81,"charging":false,"thermal":"none","cpu_pct":7.5}"#,
        r#"{"t":"phone_info","phone_name":"Desk phone"}"#,
        r#"{"t":"meter","score":68,"label":"good","tip":"ready"}"#,
        r#"{"t":"gesture","kind":"poke","acted":true}"#,
        r#"{"t":"request_start"}"#,
        r#"{"t":"error","code":"bad_message","message":"Unknown audio format 3."}"#,
        r#"{"t":"bye","reason":"user_quit"}"#,
    ].into_iter().map(str::to_owned).collect()
}
pub fn generate() -> String {
    // X25519's independent known-answer inputs are RFC 7748 section 6.1.
    let pc_seed: [u8; 32] =
        unhex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
            .try_into()
            .unwrap();
    let phone_seed: [u8; 32] =
        unhex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")
            .try_into()
            .unwrap();
    let pc = Ephemeral::from_seed(pc_seed, [0x11; 32]);
    let phone = Ephemeral::from_seed(phone_seed, [0x22; 32]);
    let commitment = crypto::commit(&pc.public, &pc.nonce);
    let bodies: Vec<_> = [
        Handshake::PairHello {
            v: 1,
            pc_id: Bytes([1; 16]),
            pc_name: "Desk".into(),
            app: "veyl-desktop/0.0.1".into(),
            commit: Bytes(commitment),
        },
        Handshake::PairReply {
            v: 1,
            phone_id: Bytes([2; 16]),
            phone_name: "Phone".into(),
            model: "Test phone".into(),
            app: "veyl-android/0.0.1".into(),
            e_pub: Bytes(phone.public),
            nonce: Bytes(phone.nonce),
        },
        Handshake::PairReveal {
            e_pub: Bytes(pc.public),
            nonce: Bytes(pc.nonce),
        },
    ]
    .iter()
    .map(|message| String::from_utf8(message.frame().unwrap().body).unwrap())
    .collect();
    let shared = pc.shared(&phone.public).unwrap();
    assert_eq!(
        hex(shared.expose()),
        "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"
    );
    let keys = PairKeys::derive(
        &shared,
        &bodies
            .iter()
            .map(|body| body.as_bytes())
            .collect::<Vec<_>>(),
    );
    let pair = json!({ "pc_private":hex(&pc_seed), "phone_private":hex(&phone_seed), "pc_public":hex(&pc.public), "phone_public":hex(&phone.public),
        "pc_nonce":hex(&pc.nonce), "phone_nonce":hex(&phone.nonce), "shared":hex(shared.expose()), "commit":hex(&commitment), "bodies":bodies,
        "transcript":hex(&keys.transcript), "okm":hex(&[keys.link.expose().as_slice(),keys.sas_key.expose()].concat()), "k_link":hex(keys.link.expose()), "k_sas":hex(keys.sas_key.expose()),
        "sas":keys.sas(), "confirm_pc":hex(&keys.confirmation(true)), "confirm_phone":hex(&keys.confirmation(false)) });
    let session_pc_seed = crypto::hash(&[b"Veyl session PC fixture"]);
    let session_phone_seed = crypto::hash(&[b"Veyl session phone fixture"]);
    let pc = Ephemeral::from_seed(session_pc_seed, [0x33; 32]);
    let phone = Ephemeral::from_seed(session_phone_seed, [0x44; 32]);
    let bodies: Vec<_> = [
        Handshake::Hello {
            v: 1,
            pc_id: Bytes([1; 16]),
            app: "veyl-desktop/0.0.1".into(),
            e_pub: Bytes(pc.public),
            nonce: Bytes(pc.nonce),
        },
        Handshake::HelloReply {
            v: 1,
            phone_id: Bytes([2; 16]),
            app: "veyl-android/0.0.1".into(),
            e_pub: Bytes(phone.public),
            nonce: Bytes(phone.nonce),
        },
    ]
    .iter()
    .map(|message| String::from_utf8(message.frame().unwrap().body).unwrap())
    .collect();
    let shared = pc.shared(&phone.public).unwrap();
    let session_keys = SessionKeys::derive(
        &shared,
        &keys.link,
        &bodies
            .iter()
            .map(|body| body.as_bytes())
            .collect::<Vec<_>>(),
    );
    let controls = controls();
    let audio = AudioBlock {
        stream: 0,
        channels: 2,
        sample_rate: 48000,
        seq: 0,
        first_frame: 0,
        capture_ns: -1234567890,
        frames: 4,
        flags: 12,
        pre_rms_dbfs: -24.0,
        pre_peak: 0.75,
        samples: vec![-32768, -123, 0, 100, 12345, 32767, -1, 1],
    };
    let audio_plain = audio.encode().unwrap();
    let ping = Ping(1234567890123);
    let pong = Pong {
        pc_ns: ping.0,
        phone_ns: -1234567890,
    };
    let mut pc_channel = crypto::SealedChannel::new(
        crypto::Secret::new(*session_keys.pc_to_phone.expose()),
        crypto::Secret::new(*session_keys.phone_to_pc.expose()),
    );
    let mut phone_channel = crypto::SealedChannel::new(
        crypto::Secret::new(*session_keys.phone_to_pc.expose()),
        crypto::Secret::new(*session_keys.pc_to_phone.expose()),
    );
    let mut frames = |pc_role: bool| -> Vec<Value> {
        let payloads = if pc_role {
            vec![
                (CONTROL, controls[1].as_bytes().to_vec()),
                (PING, ping.encode().to_vec()),
                (0x7e, vec![0, 1, 2, 255]),
            ]
        } else {
            vec![
                (CONTROL, controls[0].as_bytes().to_vec()),
                (AUDIO, audio_plain[1..].to_vec()),
                (PONG, pong.encode()),
            ]
        };
        let channel = if pc_role {
            &mut pc_channel
        } else {
            &mut phone_channel
        };
        payloads.iter().enumerate().map(|(counter, (kind, payload))| {
            let frame: Frame = channel.seal(*kind, payload).unwrap(); json!({"type":kind,"payload":hex(payload),"counter":counter,"wire":hex(&frame.wire().unwrap())})
        }).collect()
    };
    let pc_frames = frames(true);
    let phone_frames = frames(false);
    let session = json!({ "pc_private":hex(&session_pc_seed), "phone_private":hex(&session_phone_seed), "pc_public":hex(&pc.public), "phone_public":hex(&phone.public), "pc_nonce":hex(&pc.nonce), "phone_nonce":hex(&phone.nonce),
        "shared":hex(shared.expose()), "bodies":bodies, "transcript":hex(&crypto::hash(&bodies.iter().map(|body| body.as_bytes()).collect::<Vec<_>>())),
        "okm":hex(&[session_keys.pc_to_phone.expose().as_slice(),session_keys.phone_to_pc.expose()].concat()), "k_pc_to_phone":hex(session_keys.pc_to_phone.expose()), "k_phone_to_pc":hex(session_keys.phone_to_pc.expose()), "pc_to_phone":pc_frames,"phone_to_pc":phone_frames });
    let vectors = json!({"protocol_version":1,"pair":pair,"session":session,"controls":controls,"audio":{"fields":audio,"plain":hex(&audio_plain)},"ping":{"pc_ns":ping.0,"payload":hex(&ping.encode())},"pong":{"pc_ns":pong.pc_ns,"phone_ns":pong.phone_ns,"payload":hex(&pong.encode())}});
    serde_json::to_string_pretty(&vectors).unwrap() + "\n"
}
