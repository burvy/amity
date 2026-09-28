use avian3d::prelude::*;
use bevy::{ecs::entity::MapEntities, prelude::*};
use serde::{Deserialize, Serialize};

use super::config::{PLAYER_HEIGHT, PLAYER_WIDTH};

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
    /// Whether the player is holding crouch (C). While flying: move down
    pub descend: bool,
    /// The selected hotbar slot, 0..HOTBAR_SLOTS. Sent every tick like the rest,
    /// since it's a state (which slot), not an event
    pub slot: u8,
    /// Whether the player's flashlight is on. Only for show, but sent like
    /// the rest so every player sees everyone's beam
    pub flashlight: bool,
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
/// client and server.
///
/// The body is kinematic: physics never moves it or pushes it. Only
/// `movement::player_motion` does, sweeping the box along its velocity and
/// sliding along whatever it hits (avian's move-and-slide). So a player's whole
/// state is their `Position` and `LinearVelocity`, which lightyear already
/// rolls back, and the client predicts exactly what the server does.
///
/// The box is upright and never turns: the look direction lives only in the
/// inputs. A turning box would swing its corners into walls.
#[derive(Bundle)]
pub struct PlayerBody {
    body: RigidBody,
    collider: Collider,
    /// Tells avian not to move the body by its velocity; movement does that
    custom_integration: CustomPositionIntegration,
    /// No early contacts with things the body is heading toward. Those would
    /// shove props on the physics' own terms rather than movement's
    margin: SpeculativeMargin,
}

impl Default for PlayerBody {
    fn default() -> Self {
        Self {
            body: RigidBody::Kinematic,
            collider: player_collider(),
            custom_integration: CustomPositionIntegration,
            margin: SpeculativeMargin(0.0),
        }
    }
}

/// The body's shape: an upright box around its centre
pub fn player_collider() -> Collider {
    Collider::cuboid(PLAYER_WIDTH, PLAYER_HEIGHT, PLAYER_WIDTH)
}
