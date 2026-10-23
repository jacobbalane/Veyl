use crate::{sampling::Noise, scene::*, SCENARIOS};
use std::io;
pub fn duration(name: &str) -> Option<f32> {
    Some(match name {
        "still" => 60.0,
        "hover_drift" | "walk_by" | "typing" => 300.0,
        "wave" | "drift_50ppm" | "drift_200ppm" | "pc_mode" | "pc_bluetooth" => 120.0,
        "poke_varied" => 100.0,
        other if SCENARIOS.contains(&other) => 40.0,
        _ => return None,
    })
}
pub fn scene(name: &str, seed: u64, seconds: Option<f32>) -> io::Result<Scene> {
    let default = duration(name)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Unknown acoustic scenario."))?;
    let mut scene = Scene::phone(seed, seconds.unwrap_or(default));
    let mut pokes: Vec<_> = (0..if name == "poke_varied" { 30 } else { 10 })
        .map(|i| Poke {
            from: [0.0, 0.0, 0.2],
            depth: 0.05,
            turn_seconds: 5.0 + i as f64 * 3.0,
            approach_ms: 150.0,
            hold_ms: 30.0,
            retreat_ms: 200.0,
        })
        .collect();
    if name == "poke_varied" {
        let mut noise = Noise::new(seed);
        let mut unit = || (noise.sample() as f64 + 1.0) * 0.5;
        for poke in &mut pokes {
            poke.from[2] = 0.12 + 0.18 * unit();
            poke.depth = 0.025 + 0.055 * unit();
            poke.approach_ms = 100.0 + 200.0 * unit();
        }
    }
    match name {
        "still" | "typing" | "silence" | "blocked" | "latency_200ms" => {}
        "hover_drift" => scene.movers.push(Mover::Hover {
            position: [0.0, 0.0, 0.2],
            jitter_mm: 30.0,
        }),
        "wave" => scene.movers.push(Mover::Wave {
            height: 0.2,
            amplitude: 0.05,
            hz: 1.0,
        }),
        "walk_by" => scene.movers.push(Mover::Walk {
            distance_range: [0.6, 1.2],
            speed: 0.12,
        }),
        _ => scene.movers.push(Mover::Pokes(pokes)),
    }
    match name {
        "typing" | "typing_pokes" => scene.room_noise.push(RoomNoise::Typing { rate_hz: 8.0 }),
        "dull_music" => scene.music = Music::Dull { seed },
        "compressed16k" => scene.music = Music::Compressed16k { seed },
        "gappy" => scene.music = Music::Gappy { seed },
        "silence" => scene.music = Music::Silence,
        "blocked" => scene.events.push(SceneEvent::CaptureBlocked {
            at: 0.0,
            blocked: true,
        }),
        "volume_steps" => {
            for i in 1..=(scene.seconds / 7.0) as usize {
                scene.events.push(SceneEvent::Volume {
                    at: i as f64 * 7.0,
                    gain: if i % 2 == 1 { 2.0 } else { 0.5 },
                });
            }
        }
        "drift_50ppm" => scene.clock_drift_ppm = 50.0,
        "drift_200ppm" => scene.clock_drift_ppm = 200.0,
        "latency_200ms" => scene.output_latency_ms = 200.0,
        "dropouts" => {
            let mut noise = Noise::new(seed ^ 0xd00d_1234_5678_9012);
            for i in 0..(scene.seconds * 100.0).round() as usize {
                if noise.sample() > 0.99 {
                    scene.dropouts.push((i as f64 / 100.0, 0.01));
                }
            }
            scene.dropouts.push((15.0, 0.6));
        }
        "stereo_speakers" => {
            scene.speakers = vec![
                Transducer {
                    position: [0.02, -0.075, 0.0],
                    channel: Channel::Right,
                },
                Transducer {
                    position: [0.0, 0.075, 0.0],
                    channel: Channel::Left,
                },
            ]
        }
        "pc_mode" | "pc_bluetooth" => {
            scene.speakers = vec![
                Transducer {
                    position: [-0.2, 0.3, 0.0],
                    channel: Channel::Left,
                },
                Transducer {
                    position: [0.2, 0.3, 0.0],
                    channel: Channel::Right,
                },
            ];
            scene.mics = vec![
                Transducer {
                    position: [-0.02, 0.0, 0.0],
                    channel: Channel::Mix,
                },
                Transducer {
                    position: [0.02, 0.0, 0.0],
                    channel: Channel::Mix,
                },
            ];
            scene.clock_drift_ppm = 150.0;
            if name == "pc_bluetooth" {
                scene.output_latency_ms = 350.0;
            }
            if let Mover::Pokes(pokes) = &mut scene.movers[0] {
                for poke in pokes {
                    poke.from[2] = 0.25;
                }
            }
        }
        _ => {}
    }
    scene.validate()?;
    Ok(scene)
}
