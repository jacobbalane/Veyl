use super::*;
use crate::echo::{DirectPeak, ReferenceMode};

fn status() -> SourceStatus {
    SourceStatus {
        capture_running: true,
        source: SoundSource::TestSound,
        ..SourceStatus::default()
    }
}
fn echo() -> EchoFrame {
    EchoFrame {
        mode: ReferenceMode::Mono,
        mode_changed: false,
        coherence: 1.0,
        bandwidth_hz: 12000.0,
        reference_weights: [1.0, 0.0],
        reference_cross: Default::default(),
        path_mismatch: 0.0,
        predicted_power: 100.0,
        residual_power: 1.0,
        impulse: false,
        impulse_fraction: 0.0,
        direct: DirectPeak::default(),
    }
}
fn feed(
    meter: &mut Meter,
    index: u64,
    reference_db: Option<f32>,
    mic_db: Option<f32>,
    clipped: bool,
    gap: bool,
) -> Option<MeterReading> {
    let frames = meter.rate as usize / 100;
    for (stream, db) in [
        (Stream::Reference, reference_db),
        (Stream::Microphone, mic_db),
    ] {
        let amplitude = db.map_or(0, |v| (32768.0 * 10.0_f32.powf(v / 20.0)).round() as i16);
        let samples: Vec<_> = (0..frames)
            .map(|i| if i % 2 == 0 { amplitude } else { -amplitude })
            .collect();
        let block = BlockIn {
            stream,
            first_frame: index * frames as u64,
            capture_ns: None,
            flags: BlockFlags(
                u16::from(gap)
                    | if clipped && stream == Stream::Microphone {
                        BlockFlags::CLIPPED
                    } else {
                        0
                    },
            ),
            pre_rms_dbfs: if stream == Stream::Reference && db.is_some() {
                -12.0
            } else {
                db.unwrap_or(-120.0)
            },
            pre_peak: if db.is_some() { 0.5 } else { 0.0 },
            samples: &samples,
        };
        meter.observe_block(&block, 1);
    }
    let frame = (index + 1) * frames as u64;
    meter.observe_echo(frame - 1, echo());
    meter.tick(frame, true, false)
}

#[test]
fn bright_clear_sound_converges_at_both_rates_with_frame_timed_readings_and_no_startup_score() {
    for rate in [44100, 48000] {
        let mut meter = Meter::new(rate, 256, false, 20.0);
        assert_eq!(meter.reading().score, 0);
        meter.set_status(status()).unwrap();
        let mut updates = vec![];
        for i in 0..400 {
            if let Some(value) = feed(&mut meter, i, Some(-30.0), Some(-38.0), false, false) {
                updates.push(value);
            }
        }
        assert_eq!(updates.len(), 40);
        assert!(updates
            .windows(2)
            .all(|pair| pair[1].frame - pair[0].frame == rate as u64 / 10));
        let reading = meter.reading();
        assert_eq!(reading.score, 100);
        assert_eq!(reading.label, MeterLabel::Great);
        assert_eq!(reading.tip, Tip::Ready);
        assert!((reading.brightness_db.unwrap() + 18.0).abs() < 0.03);
        assert_eq!(reading.detail_cm, Some(1.4));
        assert!(reading.clarity_db >= 19.9 && reading.steady_fraction == 1.0);
    }
}

#[test]
fn blocked_and_silent_capture_are_distinguished_and_gaps_do_not_count_as_two_seconds_of_zeros() {
    for quiet_microphone in [false, true] {
        let mut meter = Meter::new(48000, 256, false, 20.0);
        meter
            .set_status(SourceStatus {
                source: SoundSource::Capture,
                ..status()
            })
            .unwrap();
        let mut first = None;
        for i in 0..300 {
            if let Some(value) = feed(
                &mut meter,
                i,
                None,
                if quiet_microphone { None } else { Some(-40.0) },
                false,
                false,
            ) {
                if matches!(value.tip, Tip::CaptureBlocked | Tip::NothingPlaying) {
                    first.get_or_insert(value.frame);
                }
            }
        }
        assert_eq!(meter.reading().score, 0);
        assert_eq!(meter.reading().label, MeterLabel::None);
        assert_eq!(
            meter.reading().tip,
            if quiet_microphone {
                Tip::NothingPlaying
            } else {
                Tip::CaptureBlocked
            }
        );
        assert!(first.unwrap() <= 3 * 48000);
        assert!(!meter.reference_active());
    }
    let mut meter = Meter::new(48000, 256, false, 20.0);
    meter
        .set_status(SourceStatus {
            source: SoundSource::Capture,
            ..status()
        })
        .unwrap();
    for i in 0..400 {
        feed(&mut meter, i, None, Some(-40.0), false, true);
    }
    assert_ne!(meter.reading().tip, Tip::CaptureBlocked);
}

#[test]
fn route_capture_and_microphone_safety_override_good_audio_in_tip_order() {
    for (pc, source, tip) in [
        (
            false,
            SourceStatus {
                capture_running: false,
                ..status()
            },
            Tip::CaptureOff,
        ),
        (
            false,
            SourceStatus {
                route: SoundRoute::External,
                mic_silenced: true,
                ..status()
            },
            Tip::NotSpeaker,
        ),
        (
            true,
            SourceStatus {
                route: SoundRoute::Headphones,
                mic_available: false,
                ..status()
            },
            Tip::PcHeadphones,
        ),
        (
            true,
            SourceStatus {
                mic_available: false,
                ..status()
            },
            Tip::PcNoMic,
        ),
        (
            false,
            SourceStatus {
                mic_silenced: true,
                ..status()
            },
            Tip::MicSilenced,
        ),
    ] {
        let mut meter = Meter::new(48000, 256, pc, 20.0);
        meter.set_status(source).unwrap();
        for i in 0..350 {
            feed(&mut meter, i, Some(-30.0), Some(-38.0), false, false);
        }
        assert_eq!(meter.reading().tip, tip);
        assert_eq!(meter.reading().score, 0);
        assert!(!meter.status().permits_detection(pc));
    }
    let mut meter = Meter::new(48000, 256, true, 20.0);
    meter
        .set_status(SourceStatus {
            route: SoundRoute::Bluetooth,
            ..status()
        })
        .unwrap();
    for i in 0..350 {
        feed(&mut meter, i, Some(-30.0), Some(-38.0), false, false);
    }
    assert_eq!(meter.reading().tip, Tip::PcBluetooth);
    assert!(meter.status().permits_detection(true));
    assert!(!meter.status().permits_detection(false));
}

#[test]
fn clipping_expires_after_one_second_and_silence_reduces_the_rolling_steady_factor() {
    let mut meter = Meter::new(48000, 256, false, 20.0);
    meter.set_status(status()).unwrap();
    for i in 0..350 {
        feed(&mut meter, i, Some(-30.0), Some(-38.0), false, false);
    }
    for i in 350..400 {
        feed(&mut meter, i, Some(-30.0), Some(-38.0), i == 350, false);
    }
    assert!(meter.reading().mic_clip > 0.001);
    assert!(meter.reading().factors.volume <= 0.6);
    assert_eq!(meter.reading().tip, Tip::Clipping);
    for i in 400..460 {
        feed(&mut meter, i, Some(-30.0), Some(-38.0), false, false);
    }
    assert_eq!(meter.reading().mic_clip, 0.0);
    for i in 460..1060 {
        feed(
            &mut meter,
            i,
            if (i / 100) % 2 == 0 {
                Some(-30.0)
            } else {
                None
            },
            Some(-38.0),
            false,
            false,
        );
    }
    assert!(meter.reading().factors.steady < 0.6);
    assert!(meter.reading().steady_fraction < 0.7);
    assert!(
        meter.reference.len() <= 10 && meter.microphone.len() <= 100 && meter.steady.len() == 30
    );
    assert!(meter
        .set_status(SourceStatus {
            media_volume: Some(f32::NAN),
            ..status()
        })
        .is_err());
}
