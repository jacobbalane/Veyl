//! Engine properties use one test binary so its allocator cannot affect other tests.
use rustfft::num_complex::Complex;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
};
use veyl_dsp::{
    config::EngineConfig,
    echo::{DirectPeak, EchoFrame, ReferenceMode},
    motion::MotionFeatures,
    BlockFlags, BlockIn, Engine, EngineEvent, EngineSnapshot, EngineState, MeterFactors,
    MeterLabel, MeterReading, PokeEvent, SoundSource, SourceStatus, Stream, StreamFormat, Tip,
    Warning,
};
use veyl_sim::{basic::quantize, scenarios, scene};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Allocations {
    alloc: u64,
    zeroed: u64,
    realloc: u64,
    dealloc: u64,
}
thread_local! { static COUNTS: Cell<Option<Allocations>> = const { Cell::new(None) }; }
fn record(kind: u8) {
    let _ = COUNTS.try_with(|counter| {
        if let Some(mut counts) = counter.get() {
            match kind {
                0 => counts.alloc += 1,
                1 => counts.zeroed += 1,
                2 => counts.realloc += 1,
                _ => counts.dealloc += 1,
            }
            counter.set(Some(counts));
        }
    });
}
struct CountedSystem;
unsafe impl GlobalAlloc for CountedSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            record(0);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            record(1);
        }
        p
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(p, layout, size) };
        if !p.is_null() {
            record(2);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        record(3);
        unsafe { System.dealloc(p, layout) };
    }
}
#[global_allocator]
static ALLOCATOR: CountedSystem = CountedSystem;
fn counted(call: impl FnOnce()) -> Allocations {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            COUNTS.with(|c| c.set(None));
        }
    }
    assert!(COUNTS
        .with(|c| c.replace(Some(Allocations::default())))
        .is_none());
    let reset = Reset;
    call();
    let result = COUNTS.with(Cell::take).unwrap();
    drop(reset);
    result
}

// Explicit fields compare float bits, avoid struct padding and reject uncovered new fields.
trait Bits {
    fn bits(&self, out: &mut Vec<u64>);
}
macro_rules! scalar_bits {
    ($($ty:ty),+) => { $(impl Bits for $ty { fn bits(&self, out: &mut Vec<u64>) { out.push(*self as u64); } })+ };
}
scalar_bits!(
    bool,
    u8,
    u64,
    i64,
    usize,
    EngineState,
    Warning,
    ReferenceMode,
    MeterLabel,
    Tip
);
impl Bits for f32 {
    fn bits(&self, out: &mut Vec<u64>) {
        out.push(self.to_bits() as u64);
    }
}
impl Bits for f64 {
    fn bits(&self, out: &mut Vec<u64>) {
        out.push(self.to_bits());
    }
}
impl<T: Bits> Bits for Option<T> {
    fn bits(&self, out: &mut Vec<u64>) {
        self.is_some().bits(out);
        if let Some(value) = self {
            value.bits(out);
        }
    }
}
impl<T: Bits, const N: usize> Bits for [T; N] {
    fn bits(&self, out: &mut Vec<u64>) {
        for value in self {
            value.bits(out);
        }
    }
}
impl<T: Bits> Bits for Vec<T> {
    fn bits(&self, out: &mut Vec<u64>) {
        self.len().bits(out);
        for value in self {
            value.bits(out);
        }
    }
}
macro_rules! struct_bits {
    ($ty:ty { $($field:ident),+ $(,)? }) => {
        impl Bits for $ty { fn bits(&self, out: &mut Vec<u64>) {
            let Self { $($field),+ } = self;
            $($field.bits(out);)+
        } }
    };
}
struct_bits!(Complex<f32> { re, im });
struct_bits!(DirectPeak {
    position,
    variance,
    energy
});
struct_bits!(MotionFeatures {
    frame,
    valid,
    settling,
    z_max,
    peak_floor,
    peak_energy,
    direct_energy,
    peakiness,
    mean_energy,
    r_peak_cm,
    distance_cm,
    present,
    changed
});
struct_bits!(MeterFactors {
    high,
    detail,
    volume,
    steady,
    clarity
});
struct_bits!(MeterReading {
    frame,
    score,
    label,
    tip,
    factors,
    brightness_db,
    bandwidth_hz,
    mic_band_dbfs,
    clarity_db,
    steady_fraction,
    mic_clip,
    detail_cm
});
struct_bits!(PokeEvent {
    turn_frame,
    detect_frame,
    distance_cm,
    depth_cm,
    confidence
});
struct_bits!(EchoFrame {
    mode,
    mode_changed,
    coherence,
    bandwidth_hz,
    reference_weights,
    reference_cross,
    path_mismatch,
    predicted_power,
    residual_power,
    impulse,
    impulse_fraction,
    direct
});
struct_bits!(EngineSnapshot {
    state,
    frame,
    lag_frames,
    drift_ppm,
    direct_residual,
    hand,
    motion,
    echo,
    meter,
    suspended,
    paused,
    processed_hops,
    invalid_hops
});
impl Bits for EngineEvent {
    fn bits(&self, out: &mut Vec<u64>) {
        match self {
            Self::State(state) => {
                0_u8.bits(out);
                state.bits(out);
            }
            Self::Aligned {
                lag_frames,
                lag_ms,
                psr,
            } => {
                1_u8.bits(out);
                lag_frames.bits(out);
                lag_ms.bits(out);
                psr.bits(out);
            }
            Self::Hand {
                present,
                distance_cm,
            } => {
                2_u8.bits(out);
                present.bits(out);
                distance_cm.bits(out);
            }
            Self::Features(value) => {
                3_u8.bits(out);
                value.bits(out);
            }
            Self::Poke(value) => {
                4_u8.bits(out);
                value.bits(out);
            }
            Self::Meter(value) => {
                5_u8.bits(out);
                value.bits(out);
            }
            Self::Warning(value) => {
                6_u8.bits(out);
                value.bits(out);
            }
        }
    }
}

#[test]
fn warmed_engine_allocates_nothing_and_replays_all_outputs_bit_identically() {
    // A disabled or broken counter must not turn a zero-allocation check into a false pass.
    let calibration = counted(|| {
        let mut probe = Vec::with_capacity(black_box(1));
        probe.push(black_box(1_u8));
        probe.reserve_exact(black_box(1024));
        let zeroed = vec![0_u64; black_box(1024)];
        black_box((&probe, &zeroed));
    });
    assert!(
        calibration.alloc > 0
            && calibration.zeroed > 0
            && calibration.realloc > 0
            && calibration.dealloc >= 2
    );

    let scene = scenarios::scene("drift_200ppm", 1, Some(20.0)).unwrap();
    let rendered = scene::render(&scene).unwrap();
    assert_eq!(
        (
            rendered.sample_rate,
            rendered.ref_channels,
            rendered.mic_channels
        ),
        (48000, 2, 2)
    );
    // Rendering and quantization finish before any counted Engine call; PCM is never printed.
    let pcm: [Vec<i16>; 2] = [
        rendered.reference.iter().map(|v| quantize(*v)).collect(),
        rendered.mic.iter().map(|v| quantize(*v)).collect(),
    ];
    let format = StreamFormat {
        sample_rate: 48000,
        ref_channels: 2,
        mic_channels: 2,
    };
    let candidates = [
        ("specified", EngineConfig::specified()),
        (
            "phone",
            EngineConfig {
                fast_seconds: 0.005,
                infer_single_path: true,
                resample_reference: true,
                ..EngineConfig::specified()
            },
        ),
        (
            "residual",
            EngineConfig {
                infer_single_path: true,
                resample_reference: true,
                residual_motion: true,
                floor_relative: 1e-8,
                impulse_ratio: 12.0,
                ..EngineConfig::specified()
            },
        ),
        ("default", EngineConfig::default()),
    ];
    for (candidate, config) in candidates {
        assert_eq!(config.recheck_seconds, 5.0);
        let make_engine = || {
            let mut engine = Engine::new(config.clone(), format).unwrap();
            engine
                .set_source_status(SourceStatus {
                    capture_running: true,
                    source: SoundSource::TestSound,
                    ..SourceStatus::default()
                })
                .unwrap();
            engine
        };
        let mut engines = [make_engine(), make_engine()];
        let mut events = [Vec::with_capacity(64), Vec::with_capacity(64)];
        let mut outputs = [Vec::with_capacity(512), Vec::with_capacity(512)];
        let mut first_lock = None;
        let mut warm_blocks = 0;
        let mut warm_features = 0;
        for (index, meta) in rendered.blocks.iter().enumerate() {
            let warm = meta.first_frame >= 48000 * 10;
            let first = meta.first_frame as usize * 2;
            for i in 0..2 {
                events[i].clear();
                let block = BlockIn {
                    stream: if meta.stream == 0 {
                        Stream::Reference
                    } else {
                        Stream::Microphone
                    },
                    first_frame: meta.first_frame,
                    capture_ns: Some(meta.capture_ns),
                    flags: BlockFlags(meta.flags),
                    pre_rms_dbfs: meta.pre_rms_dbfs,
                    pre_peak: meta.pre_peak,
                    samples: &pcm[meta.stream as usize][first..first + 960],
                };
                let calls = || {
                    engines[i].push(block).unwrap();
                    engines[i].process(&mut events[i]).unwrap();
                };
                if warm {
                    assert_eq!(
                        counted(calls),
                        Allocations::default(),
                        "Engine allocated at block {index}, candidate={candidate}"
                    );
                } else {
                    calls();
                }
                assert_eq!(events[i].capacity(), 64, "Reusable events grew");
                let snapshot = engines[i].snapshot();
                if warm {
                    assert_eq!(snapshot.state, EngineState::Running);
                    assert_eq!(snapshot.invalid_hops, 0);
                }
                outputs[i].clear();
                events[i].bits(&mut outputs[i]);
                snapshot.bits(&mut outputs[i]);
            }
            assert!(
                outputs[0] == outputs[1],
                "Bitwise output differs at block {index}, candidate={candidate}"
            );
            if events[0]
                .iter()
                .any(|e| matches!(e, EngineEvent::Aligned { .. }))
            {
                first_lock.get_or_insert(meta.first_frame);
            }
            if warm {
                warm_blocks += 1;
                warm_features += events[0]
                    .iter()
                    .filter(|e| matches!(e, EngineEvent::Features(f) if f.valid && !f.settling))
                    .count();
            }
        }
        // Stable rechecks emit no event. Early lock plus a continuous 20 s timeline and the
        // 5 s interval includes at least two scheduled rechecks after the 10 s warm-up.
        assert!(first_lock.is_some_and(|frame| frame < 48000 * 5));
        assert_eq!(warm_blocks, 2000);
        assert!(
            warm_features > 1500,
            "The property checks must process actual valid hops"
        );
        eprintln!("candidate={candidate}: {warm_blocks} warmed blocks, {warm_features} feature hops, zero allocations and exact output replay");
    }
}
