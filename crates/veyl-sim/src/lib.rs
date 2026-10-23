//! Deterministic acoustic inputs. M3's fake phone uses poke_basic; M4 expands the catalogue.
pub mod basic;
mod sampling;
pub mod scenarios;
pub mod scene;
pub use basic::{poke_basic, BlockMeta, PokeTruth, Rendered, TruthSample};
pub const SCENARIOS: &[&str] = &[
    "still",
    "poke_basic",
    "poke_varied",
    "hover_drift",
    "wave",
    "walk_by",
    "typing",
    "typing_pokes",
    "volume_steps",
    "drift_50ppm",
    "drift_200ppm",
    "dropouts",
    "dull_music",
    "compressed16k",
    "gappy",
    "blocked",
    "silence",
    "stereo_speakers",
    "latency_200ms",
    "pc_mode",
    "pc_bluetooth",
];
