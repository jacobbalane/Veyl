//! Veyl Link v1. The phone accepts TCP; the PC connects out.
//! State machines accept monotonic milliseconds and never perform I/O or log secrets.
pub mod audio;
pub mod crypto;
pub mod flow;
mod frames;
pub mod messages;
pub mod pairing;
pub mod session;
pub mod transport;
pub mod vectors;
pub use frames::*;

pub(crate) fn invalid(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}
