//! Aim
//! What a player is looking at, shared by every "use what I'm looking at"
//! action: interacting, editing blocks, and later attacking.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::config::EYE_HEIGHT;

/// The first collider a player's look ray hits
pub struct Aim {
    pub entity: Entity,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

/// Where a player's eye is, given their body position
pub fn eye(position: Vec3) -> Vec3 {
    position + Vec3::Y * EYE_HEIGHT
}

/// Casts from the player's eye along their look direction. Anything with a
/// collider blocks the ray; only the player themselves is ignored. Each action
/// decides whether `Aim::entity` is a valid target for it, and passes its own reach.
pub fn aim(
    spatial: &SpatialQuery,
    player: Entity,
    position: Vec3,
    look: Vec2,
    max_distance: f32,
) -> Option<Aim> {
    let dir = Dir3::new(Quat::from_euler(EulerRot::YXZ, look.x, look.y, 0.0) * Vec3::NEG_Z).ok()?;
    let origin = eye(position);
    let hit = spatial.cast_ray(
        origin,
        dir,
        max_distance,
        true,
        &SpatialQueryFilter::from_excluded_entities([player]),
    )?;
    Some(Aim {
        entity: hit.entity,
        point: origin + dir * hit.distance,
        normal: hit.normal,
        distance: hit.distance,
    })
}
