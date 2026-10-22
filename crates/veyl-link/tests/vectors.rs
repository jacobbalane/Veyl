use veyl_link::{
    audio::AudioBlock,
    messages::Control,
    read_frame,
    vectors::{generate, unhex},
};
#[test]
fn shared_vectors_match_the_committed_bytes() {
    assert!(generate() == include_str!("../../../protocol/test-vectors/v1.json"));
}
#[test]
fn every_control_and_sealed_fixture_parses() {
    let vectors: serde_json::Value = serde_json::from_str(&generate()).unwrap();
    for control in vectors["controls"].as_array().unwrap() {
        let parsed = Control::decode(control.as_str().unwrap().as_bytes()).unwrap();
        assert!(Control::decode(&parsed.encode().unwrap()).is_ok());
    }
    for direction in ["pc_to_phone", "phone_to_pc"] {
        for frame in vectors["session"][direction].as_array().unwrap() {
            assert!(read_frame(unhex(frame["wire"].as_str().unwrap()).as_slice()).is_ok());
        }
    }
    let mut audio = unhex(vectors["audio"]["plain"].as_str().unwrap());
    for (offset, value) in [
        (1, 2),
        (2, 0),
        (2, 5),
        (3, 2),
        (4, 0),
        (28, 0),
        (29, 17),
        (30, 128),
        (35, 127),
    ] {
        let original = audio[offset];
        audio[offset] = value;
        assert!(AudioBlock::decode(&audio).is_err());
        audio[offset] = original;
    }
}
