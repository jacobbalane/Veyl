//! Veyl Link framing only. Pairing and authenticated transport arrive in M1.
use std::io::{self, Read, Write};

pub const PROTOCOL_VERSION: u8 = 1;
pub const HANDSHAKE: u8 = 0x01;
pub const SEALED: u8 = 0x02;
pub const CONTROL: u8 = 0x10;
pub const AUDIO: u8 = 0x20;
pub const IMU: u8 = 0x21;
pub const PING: u8 = 0x30;
pub const PONG: u8 = 0x31;
pub const MAX_FRAME: usize = 1_048_576;
pub const MAX_HANDSHAKE: usize = 8192;
pub const MAX_CONTROL: usize = 65_536;

#[derive(Debug, PartialEq, Eq)]
pub struct Frame {
    pub kind: u8,
    pub body: Vec<u8>,
}

fn validate(kind: u8, size: usize) -> io::Result<()> {
    if ![HANDSHAKE, SEALED].contains(&kind)
        || size >= MAX_FRAME
        || (kind == HANDSHAKE && size > MAX_HANDSHAKE)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid Veyl frame kind or size.",
        ));
    }
    Ok(())
}

pub fn write_frame(mut writer: impl Write, frame: &Frame) -> io::Result<()> {
    validate(frame.kind, frame.body.len())?;
    writer.write_all(&((frame.body.len() + 1) as u32).to_be_bytes())?;
    writer.write_all(&[frame.kind])?;
    writer.write_all(&frame.body)
}

pub fn read_frame(mut reader: impl Read) -> io::Result<Frame> {
    let mut header = [0; 4];
    reader.read_exact(&mut header)?;
    let length = u32::from_be_bytes(header) as usize;
    if !(1..=MAX_FRAME).contains(&length) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid Veyl frame length.",
        ));
    }
    let mut kind = [0];
    reader.read_exact(&mut kind)?;
    validate(kind[0], length - 1)?;
    let mut body = vec![0; length - 1];
    reader.read_exact(&mut body)?;
    Ok(Frame {
        kind: kind[0],
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn codec_uses_big_endian_length_and_handles_consecutive_frames() {
        let frame = Frame {
            kind: HANDSHAKE,
            body: b"{}".to_vec(),
        };
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &frame).unwrap();
        assert_eq!(bytes, [0, 0, 0, 3, HANDSHAKE, b'{', b'}']);
        write_frame(&mut bytes, &frame).unwrap();
        let mut reader = bytes.as_slice();
        assert_eq!(read_frame(&mut reader).unwrap(), frame);
        assert_eq!(read_frame(&mut reader).unwrap(), frame);
        assert!(reader.is_empty());
    }
    #[test]
    fn rejects_oversized_unknown_and_truncated_frames_before_allocating() {
        for bytes in [
            vec![0, 0, 0, 0],
            vec![0xff; 4],
            vec![0, 0, 0, 1, 3],
            vec![0, 0, 0, 3, 1, b'{'],
        ] {
            assert!(read_frame(bytes.as_slice()).is_err());
        }
        let frame = Frame {
            kind: HANDSHAKE,
            body: vec![0; MAX_HANDSHAKE + 1],
        };
        let mut output = Vec::new();
        assert!(write_frame(&mut output, &frame).is_err());
        assert!(output.is_empty());
    }
}
