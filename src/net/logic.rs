use amity_protocol::net::config::SERVER_ADDR;
use bevy::prelude::*;
use lightyear::{
    netcode::{client_plugin::NetcodeConfig, Key, NetcodeClient},
    prelude::{client::InputTimelineConfig, *},
    webtransport::client::WebTransportClientIo,
};

use super::config::{CLIENT_ADDR, CLIENT_TIMEOUT_SECS, JITTER_MARGIN};

/// Handles web handshake and creating a connection to the server
pub fn connect(mut cmds: Commands) -> Result {
    let auth = Authentication::Manual {
        server_addr: SERVER_ADDR,
        // TODO: Let server assign the client id
        client_id: getrandom::u64()?,
        private_key: Key::default(),
        protocol_id: 0,
    };

    let client = cmds
        .spawn((
            Client::default(),
            LocalAddr(CLIENT_ADDR),
            PeerAddr(SERVER_ADDR),
            Link::default(),
            ReplicationReceiver,
            NetcodeClient::new(
                auth,
                NetcodeConfig {
                    client_timeout_secs: CLIENT_TIMEOUT_SECS, // controls timeout
                    ..default()
                },
            )?,
            WebTransportClientIo {
                certificate_digest: String::new(),
            },
            PredictionManager::default(),
            InputTimelineConfig::default().with_sync_config(SyncConfig {
                jitter_margin: JITTER_MARGIN,
                ..default()
            }),
        ))
        .id();

    cmds.trigger(Connect { entity: client });
    Ok(())
}
