//! Shared
//! Shared game logic/configuration between client and server

use avian3d::prelude::*;
use bevy::{ecs::query::QueryFilter, prelude::*};

use crate::{protocol, world::chunk::BLOCK_SIZE};

/// Shared walkspeed to ensure similar simulation between client and server
pub const WALKSPEED: f32 = 3.0;
/// Shared sprint speed multiplier
pub const SPRINT_MUL: f32 = 1.5;

pub const AIR_ACCEL: f32 = 1.0;

/// Shared jump velocity to ensure client-server similarity
pub const JUMP_VEL: f32 = 4.5;

/// Shared player radius for simulation similarity
pub const PLAYER_RADIUS: f32 = 0.3;
/// Shared player length for simulation similarity
/// This describes the cylinder section of the player,
/// excluding the caps. The total height is:
/// PLAYER_LENGTH + (2.0 * PLAYER_RADIUS) which is 1.8m
/// Recalculate for the doc if anything changes with these two constants
pub const PLAYER_LENGTH: f32 = 1.2;

/// Eye height for the camera for client or possible server utilities
pub const EYE_HEIGHT: f32 = 0.7;

/// Distance from player center to the top of the capsule (don't change)
pub const PLAYER_HALF_HEIGHT: f32 = PLAYER_LENGTH / 2.0 + PLAYER_RADIUS;

/// Distance to the ground to be considered grounded (tweak if needed)
pub const GROUND_TOLERANCE: f32 = 0.1;

/// Distance to consider interaction
pub const INTERACT_DISTANCE: f32 = 4.0;

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

/// Hosts agreed-upon changes between client and server about how motion inputs
/// are processed. It is essential that there are no disagreements between
/// motion as it will cause client-side rubberbanding.
pub fn apply_input(
    rotation: &mut Rotation,
    velocity: &mut LinearVelocity,
    input: &protocol::PlayerInputs,
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

/// Highest ledge a player walks onto without jumping: one block, plus a margin
pub const STEP_HEIGHT: f32 = BLOCK_SIZE + 0.05;
/// Step casts start this far above the body so the ground it rests on doesn't
/// count as an obstacle
const STEP_SKIN: f32 = 0.02;

/// Auto step-up. Runs right after the motion system, on both sides.
/// Reads the world first, then lifts players: `SpatialQuery` reads every
/// `Position`, so the two can't be borrowed at the same time.
#[allow(clippy::type_complexity)] // a ParamSet of queries reads worse as an alias
pub fn step_up<F: QueryFilter + 'static>(
    mut set: ParamSet<(
        Query<(Entity, &Position, &LinearVelocity, &ShapeHits), F>,
        SpatialQuery,
        Query<&mut Position, F>,
    )>,
) {
    let movers: Vec<(Entity, Vec3, Vec3)> = set
        .p0()
        .iter()
        .filter(|(_, _, _, hits)| !hits.is_empty())
        .map(|(entity, position, velocity, _)| (entity, position.0, velocity.0.with_y(0.0)))
        .collect();

    let lifts: Vec<(Entity, f32)> = {
        let spatial = set.p1();
        movers
            .into_iter()
            .filter_map(|(entity, position, horizontal)| {
                let dir = Dir3::new(horizontal).ok()?;
                // look slightly further than one tick of movement, so a player already
                // pressed against the ledge (zero velocity after contact) still counts
                let distance = horizontal.length() * protocol::TIMESTEP as f32 + 0.05;
                Some((
                    entity,
                    step_lift(&spatial, entity, position, dir, distance)?,
                ))
            })
            .collect()
    };

    let mut positions = set.p2();
    for (entity, lift) in lifts {
        if let Ok(mut position) = positions.get_mut(entity) {
            position.0.y += lift;
        }
    }
}

/// How far to lift a player moving along `dir` onto a ledge. None if nothing is in
/// the way, or if what's in the way is a wall rather than a step
fn step_lift(
    spatial: &SpatialQuery,
    player: Entity,
    position: Vec3,
    dir: Dir3,
    distance: f32,
) -> Option<f32> {
    let body = Collider::capsule(PLAYER_RADIUS, PLAYER_LENGTH);
    let filter = SpatialQueryFilter::from_excluded_entities([player]);
    let cast = |from: Vec3, toward: Dir3, max: f32| {
        spatial.cast_shape(
            &body,
            from,
            Quat::IDENTITY,
            toward,
            &ShapeCastConfig::from_max_distance(max),
            &filter,
        )
    };

    let feet = position + Vec3::Y * STEP_SKIN;
    let raised = position + Vec3::Y * STEP_HEIGHT;
    let blocked = cast(feet, dir, distance).is_some();
    let headroom = cast(feet, Dir3::Y, STEP_HEIGHT - STEP_SKIN).is_none();
    let clear_above = cast(raised, dir, distance).is_none();
    if !(blocked && headroom && clear_above) {
        return None;
    }
    // settle from the raised, moved-forward spot back down onto the ledge
    let drop = cast(raised + dir * distance, Dir3::NEG_Y, STEP_HEIGHT)?.distance;
    let lift = STEP_HEIGHT - drop;
    (lift > STEP_SKIN).then_some(lift)
}

fn air_strafe(velocity: &mut Vec3, wish: Vec3) {
    let Some(dir) = wish.try_normalize() else {
        return;
    };
    let along = velocity.dot(dir);
    let add = (wish.length() - along).clamp(0.0, AIR_ACCEL * protocol::TIMESTEP as f32);
    *velocity += dir * add;
}
