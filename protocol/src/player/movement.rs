//! Walking, sprinting, jumping and air strafing

use avian3d::prelude::*;
use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::input::native::ActionState;

use super::{
    config::{AIR_ACCEL, JUMP_VEL, SPRINT_MUL, WALKSPEED},
    definition::PlayerInputs,
};
use crate::net::config::TIMESTEP;

/// Moves every player matching `F` by their inputs. The server runs it for all
/// players (`With<PlayerMarker>`), the client for the ones it predicts
/// (`With<Predicted>`), so both sides share one movement system.
pub fn player_motion<F: QueryFilter + 'static>(
    mut players: Query<
        (
            &mut Rotation,
            &mut LinearVelocity,
            &ShapeHits,
            &ActionState<PlayerInputs>,
        ),
        F,
    >,
) {
    for (mut rot, mut vel, hits, action) in &mut players {
        apply_input(&mut rot, &mut vel, &action.0, !hits.is_empty());
    }
}

/// Hosts agreed-upon changes between client and server about how motion inputs
/// are processed. It is essential that there are no disagreements between
/// motion as it will cause client-side rubberbanding.
pub fn apply_input(
    rotation: &mut Rotation,
    velocity: &mut LinearVelocity,
    input: &PlayerInputs,
    grounded: bool, // grounded must be defined by user: `!hits.is_empty()`
) {
    // direction player is looking
    let look_x = Quat::from_rotation_y(input.look.x);
    // TODO: Allow the camera to look around, but only rotate the body on yaw
    *rotation = Rotation(look_x);

    // `PlayerInputs` is responsible for declaring which is forward
    let speed = WALKSPEED * if input.sprint { SPRINT_MUL } else { 1.0 };
    let wish =
        look_x * Vec3::new(input.motion.x, 0.0, input.motion.y).clamp_length_max(1.0) * speed;

    if grounded {
        velocity.0.x = wish.x;
        velocity.0.z = wish.z;
        if input.jump {
            velocity.0.y = JUMP_VEL;
        }
    } else {
        air_strafe(&mut velocity.0, wish);
    }
}

fn air_strafe(velocity: &mut Vec3, wish: Vec3) {
    let Some(dir) = wish.try_normalize() else {
        return;
    };
    let along = velocity.dot(dir);
    let add = (wish.length() - along).clamp(0.0, AIR_ACCEL * TIMESTEP as f32);
    *velocity += dir * add;
}
