use avian3d::dynamics::rigid_body::LinearVelocity;
use avian3d::physics_transform::{Position, Rotation};
use avian3d::spatial_query::{ShapeHits, SpatialQuery};
use bevy::prelude::*;
use lightyear::core::timeline::LocalTimeline;
use lightyear::connection::{client::Connected, client_of::ClientOf};
use lightyear::prelude::MessageSender;
use lightyear::prelude::input::native::ActionState;
use shooter_protocol::world::{chunk, edit};
use shooter_protocol::{aim, protocol, shared, stuff};

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
        let Some(aim) = aim::aim(
            &spatial,
            player,
            position.0,
            action.0.look,
            shared::INTERACT_DISTANCE,
        ) else {
            continue;
        };
        let Ok(mut interactable) = targets.get_mut(aim.entity) else {
            continue;
        };
        if interactable.ready(tick) {
            interactable.last_interacted = tick;
            // logic to run upon interaction
            info!(
                "{} interacted with {} at distance {}",
                player, aim.entity, aim.distance
            );
        }
    }
}

/// Applies players' held break/place input and tells every client what changed
pub fn server_edit_blocks(
    spatial: SpatialQuery,
    players: Query<
        (Entity, &Position, &ActionState<protocol::PlayerInputs>),
        With<protocol::PlayerMarker>,
    >,
    mut blocks: edit::Blocks,
    mut clients: Query<&mut MessageSender<edit::BlockChanged>, (With<ClientOf>, With<Connected>)>,
) {
    for (player, position, action) in &players {
        let Some((pos, block)) =
            edit::player_edit(&action.0, player, position.0, &spatial, &blocks)
        else {
            continue;
        };
        if blocks.set(pos, block) {
            for mut sender in &mut clients {
                sender.send::<chunk::WorldChannel>(edit::BlockChanged { pos, block });
            }
        }
    }
}
