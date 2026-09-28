use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Port 0 lets the OS pick a free port, so several clients can run on one machine
pub const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

/// Seconds without packets before the connection is dropped
pub const CLIENT_TIMEOUT_SECS: i32 = 300;

/// Ticks ahead of what the server needs that inputs are sent, instead of
/// lightyear's 1, so a slow client frame doesn't deliver inputs after the server
/// simulated them
pub const JITTER_MARGIN: f32 = 3.0;

/// The furthest back a correction from the server can roll our prediction
/// (ticks, 64 a second). Lightyear 0.30's default is 20 (about 0.3 s): a slow
/// frame or a lag spike puts the server's update further back than that, and
/// the correction is thrown away, leaving us out of step with the server.
/// This covers about 1.5 s
pub const MAX_ROLLBACK_TICKS: u16 = 100;
