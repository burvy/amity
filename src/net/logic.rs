use amity_protocol::net::config::SERVER_ADDR;
use bevy::prelude::*;
use lightyear::{
    netcode::{client_plugin::NetcodeConfig, Key, NetcodeClient},
    prelude::{client::InputTimelineConfig, *},
    webtransport::client::WebTransportClientIo,
};

use super::config::{CLIENT_ADDR, CLIENT_TIMEOUT_SECS, JITTER_MARGIN, MAX_ROLLBACK_TICKS};

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
            Client,
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
                // None: connect to `PeerAddr`, as https://<address>
                target: None,
            },
        ))
        .id();

    cmds.trigger(Connect { entity: client });
    Ok(())
}

/// Turns on prediction, and sets how far ahead of the server our inputs run.
/// Both are app-wide (there's one local timeline), so they're resources
pub fn prediction_resources() -> (PredictionManager, InputTimelineConfig) {
    let mut prediction = PredictionManager::default();
    prediction.rollback_policy.max_rollback_ticks = MAX_ROLLBACK_TICKS;
    (
        prediction,
        InputTimelineConfig::default().with_sync_config(SyncConfig {
            jitter_margin: JITTER_MARGIN,
            ..default()
        }),
    )
}
