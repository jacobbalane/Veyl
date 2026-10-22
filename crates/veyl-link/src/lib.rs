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

/// Only known protocol reasons are exposed to callers; arbitrary peer text is never logged.
#[derive(Debug)]
pub struct Rejection(&'static str);
impl Rejection {
    pub fn code(&self) -> &'static str {
        self.0
    }
}
impl std::fmt::Display for Rejection {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str(self.0)
    }
}
impl std::error::Error for Rejection {}
pub fn rejected(reason: &str) -> std::io::Error {
    let known = match reason {
        "not_paired" => "not_paired",
        "busy" => "busy",
        "version" => "version",
        "pairing_disabled" => "pairing_disabled",
        "rate_limited" => "rate_limited",
        "not_in_foreground" => "not_in_foreground",
        "user_cancelled" => "user_cancelled",
        "timeout" => "timeout",
        "mismatch" => "mismatch",
        "bad_mac" => "bad_mac",
        _ => "bad_message",
    };
    std::io::Error::new(std::io::ErrorKind::InvalidData, Rejection(known))
}
