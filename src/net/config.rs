use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Port 0 lets the OS pick a free port, so several clients can run on one machine
pub const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

/// Seconds without packets before the connection is dropped
pub const CLIENT_TIMEOUT_SECS: i32 = 300;

/// Ticks ahead of what the server needs that inputs are sent, instead of
/// lightyear's 1, so a slow client frame doesn't deliver inputs after the server
/// simulated them
pub const JITTER_MARGIN: f32 = 3.0;
