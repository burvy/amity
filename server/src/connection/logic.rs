use amity_protocol::{net::config::SERVER_BIND_ADDR, player::definition::PlayerMarker};
use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::{
    connection::{client::Connected, client_of::ClientOf, server::Start},
    netcode::{server_plugin::NetcodeConfig, NetcodeServer},
    webtransport::server::WebTransportServerIo,
};

use super::config::{CLIENT_TIMEOUT_SECS, HEARTBEAT_SECS};
#[cfg(not(feature = "dev-local"))]
use super::config::{CERT_PATH, KEY_PATH};

/// Opens the server for connections
pub fn start_server(mut cmds: Commands) -> Result {
    // real certificate instead of using digest
    #[cfg(not(feature = "dev-local"))]
    let identity = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(Identity::load_pemfiles(CERT_PATH, KEY_PATH))?;

    // offline dev: throwaway certificate generated at startup, no files and no
    // network. the client's dev-local skips validation, so contents don't matter.
    #[cfg(feature = "dev-local")]
    let identity = Identity::self_signed(["localhost", "127.0.0.1"])?;

    let server = cmds
        .spawn((
            NetcodeServer::new(NetcodeConfig {
                client_timeout_secs: CLIENT_TIMEOUT_SECS,
                ..default()
            }),
            LocalAddr(SERVER_BIND_ADDR),
            WebTransportServerIo {
                certificate: identity,
            },
        ))
        .id();

    cmds.trigger(Start { entity: server });
    Ok(())
}

/// Logging function to show connected things every minute
pub fn heartbeat(
    clients: Query<(), (With<ClientOf>, With<Connected>)>,
    senders: Query<(), With<ReplicationSender>>,
    players: Query<(), With<PlayerMarker>>,
    time: Res<Time>,
    mut next: Local<f32>,
) {
    if time.elapsed_secs() < *next {
        return;
    }

    *next = time.elapsed_secs() + HEARTBEAT_SECS;
    info!(
        "clients: {}, senders: {}, players: {}",
        clients.iter().count(),
        senders.iter().count(),
        players.iter().count(),
    )
}
