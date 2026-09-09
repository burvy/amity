//! Protocol
//! Shared networking structure between client and server.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use avian3d::{
    dynamics::{rigid_body::LinearVelocity, solver::islands::IslandSleepingPlugin},
    interpolation::PhysicsInterpolationPlugin,
    physics_transform::{PhysicsTransformPlugin, Position},
    PhysicsPlugins,
};
use bevy::{ecs::entity::MapEntities, prelude::*};
use lightyear::{
    avian3d::plugin::LightyearAvianPlugin,
    input::{config::InputConfig, native::plugin::InputPlugin},
    interpolation::registry::InterpolationRegistrationExt,
    prediction::registry::PredictionBuilderExt,
    prelude::AppComponentExt,
};
use serde::{Deserialize, Serialize};

/// Shared physics timestep between server and client
pub const TIMESTEP: f64 = 1.0 / 64.0;
/// It is best to fix a specific server IP and port, so the clients know who
/// to connect to. For clients, using port 0 allows the computer to choose a
/// random open port so that clients on the same machine won't have issues
/// registering as the same player when they connect to the server.
pub const SERVER_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(174, 175, 161, 63)), 5000);

pub const SERVER_BIND_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 5000);

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<PlayerMarker>().replicate();

        // ---
        // Having gravity set to 0 here would be confusing.
        // to let the player fall, unlock translation_y in shared.rs
        // ---

        app.component::<Position>()
            .replicate() // is sent over the link
            .predict() // is simulated by clients
            .with_rollback_condition(pos_rollback_condition)
            .add_linear_interpolation(); // visual interpolation

        // linear velocity does not require interpolation like position
        // because interpolation is only needed for visual things,
        // velocity is basically invisible
        app.component::<LinearVelocity>()
            .replicate() // is sent over the link
            .predict() // is simulated by clients
            .with_rollback_condition(vel_rollback_condition);

        app.add_plugins((
            PhysicsPlugins::default()
                .build()
                .disable::<IslandSleepingPlugin>()
                .disable::<PhysicsInterpolationPlugin>()
                .disable::<PhysicsTransformPlugin>(),
            LightyearAvianPlugin::default(), // MUST be added manually
        ));

        app.add_plugins(InputPlugin::<PlayerInputs> {
            config: InputConfig {
                rebroadcast_inputs: true,
                ..default()
            },
        });
    }
}

/// Marker used to mark something as a player (no fields)
/// This struct is bolted onto players as components, and thus must derive
/// Component.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Component)]
pub struct PlayerMarker;

/// Player input packets
/// This rides inside ActionState<PlayerInputs>, with ActionState being the
/// component. Thus, PlayerInputs does not need to derive Component.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Reflect, Default)]
pub struct PlayerInputs {
    /// A 2d vector storing raw yaw and pitch in rads (not accumulated)
    pub look: Vec2,
    /// A 2d vector representing the player's desired travel direction
    pub motion: Vec2,
    /// A bool representing whether the player should jump
    pub jump: bool,
}

/// `MapEntities` exists to satisfy the `InputPlugin`'s trait bound.
/// Even if `PlayerInputs` has no fields that contain entities,
/// `InputPlugin` still demands it.
/// Entity IDs are per world, and when they are sent over the network,
/// they must be remapped to point at the same thing the sender
/// originally had it pointing at.
impl MapEntities for PlayerInputs {
    fn map_entities<M: EntityMapper>(&mut self, _: &mut M) {}
}

fn pos_rollback_condition(this: &Position, that: &Position) -> bool {
    (this.0 - that.0).length() >= 0.01
}

fn vel_rollback_condition(this: &LinearVelocity, that: &LinearVelocity) -> bool {
    (this.0 - that.0).length() >= 0.01
}
