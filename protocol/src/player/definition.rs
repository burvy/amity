use avian3d::prelude::*;
use bevy::{ecs::entity::MapEntities, prelude::*};
use serde::{Deserialize, Serialize};

use super::config::{GROUND_TOLERANCE, PLAYER_HALF_HEIGHT, PLAYER_LENGTH, PLAYER_RADIUS};

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
    /// A 2d vector storing raw yaw and pitch in rads (accumulated)
    pub look: Vec2,
    /// A 2d vector representing the player's desired travel direction
    pub motion: Vec2,
    /// A bool representing whether the player should jump
    pub jump: bool,
    /// A bool representing whether the player is sprinting
    pub sprint: bool,
    /// A bool representing if the player is interacting
    pub interact: bool,
    /// A bool representing whether the player is holding the primary action
    /// (left click: break, later attack)
    pub primary: bool,
    /// A bool representing whether the player is holding the secondary action
    /// (right click: place, later use)
    pub secondary: bool,
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

/// Shared player body definition to ensure similar simulation between
/// client and server
#[derive(Bundle)]
pub struct PlayerBody {
    body: RigidBody,
    collider: Collider,
    locked: LockedAxes,
    ground_check: ShapeCaster,
    friction: Friction,
}

impl Default for PlayerBody {
    fn default() -> Self {
        Self {
            body: RigidBody::Dynamic,
            collider: Collider::capsule(PLAYER_RADIUS, PLAYER_LENGTH),
            locked: LockedAxes::ROTATION_LOCKED,
            ground_check: ShapeCaster::new(
                Collider::sphere(PLAYER_RADIUS * 0.9),
                Vec3::ZERO,
                Quat::IDENTITY,
                Dir3::NEG_Y,
            )
            .with_max_distance(PLAYER_HALF_HEIGHT - PLAYER_RADIUS * 0.9 + GROUND_TOLERANCE),
            friction: Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        }
    }
}
