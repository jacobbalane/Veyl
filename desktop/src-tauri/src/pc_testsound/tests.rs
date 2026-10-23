use super::*;
use crate::{engine_host::EngineHost, source::SoundSource};
use std::sync::atomic::AtomicUsize;

fn device() -> AudioDevice {
    AudioDevice {
        id: "fake-render".into(),
        name: "Synthetic endpoint".into(),
        form_factor: "speakers".into(),
        raw_supported: false,
        bluetooth: false,
        built_in: false,
    }
}
fn block(stream: u8, sequence: u32, samples: Vec<i16>) -> AudioBlock {
    AudioBlock {
        stream,
        channels: 1,
        sample_rate: 48000,
        seq: sequence,
        first_frame: sequence as u64 * 480,
        capture_ns: i64::MIN,
        frames: 480,
        flags: 4,
        pre_rms_dbfs: -12.0,
        pre_peak: 0.5,
        samples,
    }
}
#[tokio::test]
async fn explicit_check_measures_filtered_fake_streams_and_cancellation_closes_the_renderer_and_tap(
) {
    let (_, source) = EngineHost::start();
    let generation = source.next_generation();
    let callback = source.clone();
    let exits = Arc::new(AtomicUsize::new(0));
    let finished = exits.clone();
    let render: Render = Arc::new(move |_, stop, started| {
        let mut state = 7u64;
        let mut reference: Vec<f32> = (0..480 * 200)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 40) as f32 / 33554432.0 - 0.25
            })
            .collect();
        veyl_dsp::filters::PrivacyFilter::new(48000, 8000, 1)
            .unwrap()
            .process_interleaved(&mut reference)
            .unwrap();
        let reference: Vec<i16> = reference
            .iter()
            .map(|sample| (sample * 32768.0) as i16)
            .collect();
        let _ = started.send(());
        for sequence in 0..200 {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let offset = sequence * 480;
            let mic = (offset..offset + 480)
                .map(|i| if i >= 96 { reference[i - 96] / 2 } else { 0 })
                .collect();
            callback.audio(
                generation,
                block(0, sequence as u32, reference[offset..offset + 480].to_vec()),
                None,
            )?;
            callback.audio(generation, block(1, sequence as u32, mic), None)?;
        }
        finished.fetch_add(1, Ordering::AcqRel);
        Ok(())
    });
    let mut check = Check::with_renderer(
        device(),
        source.clone(),
        generation,
        render,
        Duration::from_millis(600),
    )
    .unwrap();
    assert!(source.tap(generation).is_err());
    let result = tokio::time::timeout(Duration::from_secs(5), &mut check.result)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(result.good);
    assert!(result.mean_coherence > 0.99);
    assert!((result.lag_ms - 2.0).abs() < 0.01);
    assert_eq!(exits.load(Ordering::Acquire), 1);
    check.shutdown().await;
    drop(source.tap(generation).unwrap());

    let finished = exits.clone();
    let render: Render = Arc::new(move |_, stop, started| {
        let _ = started.send(());
        while !stop.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(2));
        }
        finished.fetch_add(1, Ordering::AcqRel);
        Ok(())
    });
    let check = Check::with_renderer(
        device(),
        source.clone(),
        generation,
        render,
        Duration::from_secs(6),
    )
    .unwrap();
    check.shutdown().await;
    assert_eq!(exits.load(Ordering::Acquire), 2);
    drop(source.tap(generation).unwrap());
}
#[test]
fn holes_and_configuration_changes_fail_the_check_instead_of_reporting_poor_acoustics() {
    let mut input = Samples::new();
    let mut first = block(0, 7, vec![0; 480]);
    first.flags |= 1;
    input.push(first).unwrap();
    assert!(input.push(block(0, 9, vec![0; 480])).is_err());
    let mut changed = block(0, 8, vec![0; 480]);
    changed.sample_rate = 44100;
    assert!(input.push(changed).is_err());
    let mut gap = block(0, 8, vec![0; 480]);
    gap.flags |= 1;
    assert!(input.push(gap).is_err());
}
