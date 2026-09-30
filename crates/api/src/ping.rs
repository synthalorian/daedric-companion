//! RakNet unconnected ping — the same 33-byte probe the launcher sends to
//! measure latency/online on UDP 7777.
//!
//! Layout (recon §2): `0x01` | 8-byte timestamp | RakNet magic
//! `00ffff00fefefefefdfdfdfd12345678` | 8-byte client guid `aabbccddeeff0011`.

use std::io;
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

const MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];
const CLIENT_GUID: [u8; 8] = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11];

/// Send one unconnected ping; return round-trip latency.
pub fn raknet_ping(host: &str, port: u16, timeout: Duration) -> io::Result<Duration> {
    let addr = (host, port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no address for host"))?;

    let sock = UdpSocket::bind(("0.0.0.0", 0))?;
    sock.set_read_timeout(Some(timeout))?;

    let mut packet = Vec::with_capacity(33);
    packet.push(0x01);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    packet.extend_from_slice(&now_ms.to_be_bytes());
    packet.extend_from_slice(&MAGIC);
    packet.extend_from_slice(&CLIENT_GUID);

    let started = Instant::now();
    sock.send_to(&packet, addr)?;
    let mut buf = [0u8; 256];
    sock.recv(&mut buf)?;
    Ok(started.elapsed())
}
