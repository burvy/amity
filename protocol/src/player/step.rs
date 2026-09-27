//! Auto step-up: walking into a one-block ledge lifts the player onto it

use avian3d::prelude::*;
use bevy::{ecs::query::QueryFilter, prelude::*};

use super::config::{PLAYER_LENGTH, PLAYER_RADIUS, STEP_HEIGHT, STEP_SKIN};
use crate::net::config::TIMESTEP;

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
                let distance = horizontal.length() * TIMESTEP as f32 + 0.05;
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
