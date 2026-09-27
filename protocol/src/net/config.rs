use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Shared physics timestep between server and client
pub const TIMESTEP: f64 = 1.0 / 64.0;

/// It is best to fix a specific server IP and port, so the clients know who
/// to connect to. For clients, using port 0 allows the computer to choose a
/// random open port so that clients on the same machine won't have issues
/// registering as the same player when they connect to the server.
#[cfg(not(feature = "dev-local"))]
pub const SERVER_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(174, 175, 161, 63)), 5001);
/// Local dev: client and server run on the same machine.
#[cfg(feature = "dev-local")]
pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5001);

pub const SERVER_BIND_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 5001);
