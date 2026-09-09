//! Allow the client to directly connect to the server, receive replicated
//! information, and predict the world client side for instant response rather
//! than waiting for the server.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use avian3d::{
    dynamics::rigid_body::LinearVelocity, physics_transform::Rotation, spatial_query::ShapeHits,
};
use bevy::prelude::*;
use lightyear::{
    input::client::InputSystems,
    netcode::{client_plugin::NetcodeConfig, Key, NetcodeClient},
    prelude::{
        client::ClientPlugins,
        input::native::{ActionState, InputMarker},
        *,
    },
    webtransport::client::WebTransportClientIo,
};
use shooter_protocol::{protocol, shared};

use crate::player;

const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ClientPlugins {
            tick_duration: Duration::from_secs_f64(protocol::TIMESTEP),
        });
        // Networking
        app.add_systems(Startup, connect);

        // Client side (frame rate update)
        app.add_systems(
            Update,
            (
                add_physics,  // simulation
                draw_players, // draw visually
            ),
        );

        // Observer logic
        app.add_observer(detect_replicate_player);
        app.add_observer(detect_replicate_our_player);

        // Client-server interface (tick rate update)
        app.add_systems(
            FixedPreUpdate,
            buffer_input.in_set(InputSystems::WriteClientInputs),
        );

        // Simulate players
        app.add_systems(FixedUpdate, client_player_motion);
    }
}

/*
 * Basic connection
 */

/// Handles web handshake and creating a connection to the server
fn connect(mut cmds: Commands) -> Result {
    let auth = Authentication::Manual {
        server_addr: protocol::SERVER_ADDR,
        // TODO: Let server assign the client id
        client_id: getrandom::u64()?,
        private_key: Key::default(),
        protocol_id: 0,
    };

    let client = cmds
        .spawn((
            Client::default(),
            LocalAddr(CLIENT_ADDR),
            PeerAddr(protocol::SERVER_ADDR),
            Link::default(),
            ReplicationReceiver,
            NetcodeClient::new(
                auth,
                NetcodeConfig {
                    client_timeout_secs: 300, // controls timeout
                    ..default()
                },
            )?,
            WebTransportClientIo {
                certificate_digest: String::new(),
            },
            PredictionManager::default(),
        ))
        .id();

    cmds.trigger(Connect { entity: client });
    Ok(())
}

/*
 * On-join functions
 */

/// Insert physics onto player upon being newly predicted
fn add_physics(
    mut cmds: Commands,
    players: Query<Entity, (Added<Predicted>, With<protocol::PlayerMarker>)>,
) {
    for player in players.iter() {
        cmds.entity(player).insert(
            shared::PlayerBody::default(), // add ONCE
        );
    }
}

/// Insert player model onto player upon being newly predicted
fn draw_players(
    mut cmds: Commands,
    players: Query<Entity, (Added<Predicted>, With<protocol::PlayerMarker>)>,
) {
    for player in players.iter() {
        cmds.entity(player).queue_apply_scene(bsn! {
            Mesh3d(asset_value(Capsule3d::new(shared::PLAYER_RADIUS, shared::PLAYER_LENGTH)))
            MeshMaterial3d::<StandardMaterial>(asset_value(Color::WHITE))
        });
    }
}

/*
 * Observers
 */

/// Logic to run when any player joining is detected
fn detect_replicate_player(player: On<Add, protocol::PlayerMarker>) {
    info!("Player {:?} was replicated to me!", player.entity);
}

/// Logic to run when our designated controlled player joins
fn detect_replicate_our_player(our_player: On<Add, Controlled>, mut cmds: Commands) {
    cmds.entity(our_player.entity)
        .insert(InputMarker::<protocol::PlayerInputs>::default());
}

/*
 * Player input
 */

/// Player input for KEYBOARD for now!
/// TODO: allow input for other devices (mobile?)
/// TODO: allow configurable input controls
fn buffer_input(
    mut action: Single<
        &mut ActionState<protocol::PlayerInputs>,
        With<InputMarker<protocol::PlayerInputs>>,
    >,
    keys: Res<ButtonInput<KeyCode>>,
    look_res: ResMut<player::definition::PlayerInfo>, // yaw/pitch stored in resource
) {
    // TODO: configurable inputs
    let fwd = f32::from(keys.pressed(KeyCode::KeyW));
    let bwd = f32::from(keys.pressed(KeyCode::KeyS));
    let rwd = f32::from(keys.pressed(KeyCode::KeyD));
    let lwd = f32::from(keys.pressed(KeyCode::KeyA));

    action.0 = protocol::PlayerInputs {
        look: look_res.look,
        motion: Vec2 {
            x: rwd - lwd,
            y: bwd - fwd, // z motion is flipped in bevy
        },
        // Hold spacebar to jump repeatedly
        jump: keys.pressed(KeyCode::Space), // TODO: configurable inputs
        sprint: keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight), // TODO: configurable inputs
        interact: keys.pressed(KeyCode::KeyE), // TODO: configurable inputs
    };
}

/*
 * Simulate Motion
 */

fn client_player_motion(
    mut players: Query<
        (
            &mut Rotation,
            &mut LinearVelocity,
            &ShapeHits,
            &ActionState<protocol::PlayerInputs>,
        ),
        With<Predicted>,
    >,
) {
    for (mut rot, mut vel, hits, action) in &mut players {
        shared::apply_input(&mut rot, &mut vel, &action.0, !hits.is_empty());
    }
}
