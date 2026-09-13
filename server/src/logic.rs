use avian3d::dynamics::rigid_body::LinearVelocity;
use avian3d::physics_transform::{Position, Rotation};
use avian3d::spatial_query::{ShapeHits, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use lightyear::core::timeline::LocalTimeline;
use lightyear::prelude::input::native::ActionState;
use shooter_protocol::{protocol, shared, stuff};

/// Simulates player movement on the server
pub fn server_player_motion(
    mut players: Query<
        (
            &mut Rotation,
            &mut LinearVelocity,
            &ShapeHits,
            &ActionState<protocol::PlayerInputs>,
        ),
        With<protocol::PlayerMarker>,
    >,
) {
    for (mut rot, mut vel, hits, action) in &mut players {
        shared::apply_input(&mut rot, &mut vel, &action.0, !hits.is_empty());
    }
}

/// Raycasts from a player's eye when they are interacting
/// Interaction is called for every tick they are interacting
/// INTERACT_DISTANCE is the max interact distance
/// It is up to the other systems to handle how fast players are
/// allowed to interact
pub fn server_interact(
    spatial: SpatialQuery,
    players: Query<
        (Entity, &Position, &ActionState<protocol::PlayerInputs>),
        With<protocol::PlayerMarker>,
    >,
    mut targets: Query<&mut stuff::definition::Interactable>,
    timeline: Res<LocalTimeline>,
) {
    let tick = timeline.tick();
    for (player, position, action) in &players {
        if !action.0.interact {
            continue; // skip if not interacting
        }
        // interaction
        let rot = Quat::from_euler(EulerRot::YXZ, action.0.look.x, action.0.look.y, 0.0);
        let Ok(dir) = Dir3::new(rot * Vec3::NEG_Z) else {
            continue;
        };

        let origin = position.0 + Vec3::Y * shared::EYE_HEIGHT;
        let filter = SpatialQueryFilter::from_excluded_entities([player]);

        if let Some(hit) = spatial.cast_ray(origin, dir, shared::INTERACT_DISTANCE, true, &filter) {
            let Ok(mut interactable) = targets.get_mut(hit.entity) else {
                continue;
            };
            if interactable.ready(tick) {
                interactable.last_interacted = tick;
                // logic to run upon interaction
                info!(
                    "{} interacted with {} at distance {}",
                    player, hit.entity, hit.distance
                );
            } else {
                continue;
            }
        }
    }
}
