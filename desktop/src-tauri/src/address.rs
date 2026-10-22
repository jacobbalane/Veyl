//! One deadline covers hostname resolution and all IPv4 connection attempts.
use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::{
    net::{lookup_host, TcpStream},
    time::timeout,
};
use veyl_link::transport::Connection;

pub fn parse(value: &str) -> io::Result<(&str, u16)> {
    let value = value.trim();
    let (host, port) = value.rsplit_once(':').ok_or_else(invalid)?;
    if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid());
    }
    let port: u16 = port.parse().map_err(|_| invalid())?;
    if port == 0
        || host.is_empty()
        || host.len() > 253
        || !host.is_ascii()
        || !host
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'.')
    {
        return Err(invalid());
    }
    let invalid_hostname = host.bytes().all(|c| c.is_ascii_digit() || c == b'.')
        || host.trim_end_matches('.').split('.').any(|part| {
            part.is_empty() || part.len() > 63 || part.starts_with('-') || part.ends_with('-')
        });
    if host.parse::<Ipv4Addr>().is_err() && invalid_hostname {
        return Err(invalid());
    }
    Ok((host, port))
}
pub async fn connect(value: &str) -> io::Result<(Connection<TcpStream>, SocketAddrV4)> {
    let (host, port) = parse(value)?;
    timeout(Duration::from_secs(3), async {
        let addresses: Vec<_> = lookup_host((host, port))
            .await?
            .filter_map(|address| match address {
                std::net::SocketAddr::V4(ip)
                    if !ip.ip().is_unspecified()
                        && !ip.ip().is_multicast()
                        && *ip.ip() != Ipv4Addr::BROADCAST =>
                {
                    Some(ip)
                }
                _ => None,
            })
            .take(8)
            .collect();
        let mut failure = io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "No IPv4 address was found.",
        );
        for address in addresses {
            match TcpStream::connect(address).await {
                Ok(stream) => {
                    stream.set_nodelay(true)?;
                    return Ok((Connection::new(stream), address));
                }
                Err(error) => failure = error,
            }
        }
        Err(failure)
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Connection timed out."))?
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "Use an IPv4 address or hostname followed by a port.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_ipv4_and_hostnames_with_an_explicit_valid_port() {
        for value in [
            "192.168.1.23:47820",
            "phone.local:1",
            "PHONE-2:65535",
            "phone.local.:1234",
        ] {
            assert!(parse(value).is_ok());
        }
        for value in [
            "192.168.1.23",
            "phone:0",
            "phone:65536",
            "phone:-1",
            "http://phone:80",
            "[::1]:47820",
            "phone/path:1",
            ".phone:1",
            "phone..local:1",
            "-phone:1",
            "999.168.1.1:1",
            "phone: 80",
            ":1",
        ] {
            assert!(parse(value).is_err(), "{value}");
        }
    }
    #[tokio::test]
    async fn connects_out_to_an_ipv4_listener_and_frames_a_message() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let pc = async {
            let (mut connection, actual) = connect(&address.to_string()).await.unwrap();
            assert_eq!(actual.to_string(), address.to_string());
            connection
                .send(&veyl_link::messages::Handshake::error("busy"))
                .await
                .unwrap();
        };
        let phone = async {
            let (stream, _) = listener.accept().await.unwrap();
            let mut connection = Connection::new(stream);
            let frame = connection.receive().await.unwrap();
            assert!(
                matches!(veyl_link::messages::Handshake::parse(&frame).unwrap(),veyl_link::messages::Handshake::Error {code,..} if code=="busy")
            );
        };
        tokio::join!(pc, phone);
    }
}
