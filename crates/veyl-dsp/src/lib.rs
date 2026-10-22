//! The portable echo engine. M0 supplies source filtering and generated test sound.

pub mod filters;
pub mod testsound;

/// The engine remains idle until the processing stages are implemented in M4.
#[derive(Debug, Default)]
pub struct Engine;

impl Engine {
    pub fn new() -> Self {
        Self
    }
}
