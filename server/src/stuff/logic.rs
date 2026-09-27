use amity_protocol::{
    player::{aim, config::INTERACT_DISTANCE, definition::{PlayerInputs, PlayerMarker}},
    stuff::{definition::Interactable, ibox::definition::InteractableBox},
};
use avian3d::prelude::{Position, SpatialQuery};
use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::*;

/// The floating test box
pub fn spawn_box(mut cmds: Commands) {
    cmds.spawn((
        InteractableBox::default(),
        Interactable {
            delay_secs: 1.0,
            last_interacted: Tick(0),
        },
        Position(Vec3::new(0.0, 1.0, 0.0)),
        Replicate::to_clients(NetworkTarget::All),
    ));
}

/// Raycasts from a player's eye when they are interacting
/// Interaction is called for every tick they are interacting
/// INTERACT_DISTANCE is the max interact distance
/// It is up to the other systems to handle how fast players are
/// allowed to interact
pub fn server_interact(
    spatial: SpatialQuery,
    players: Query<(Entity, &Position, &ActionState<PlayerInputs>), With<PlayerMarker>>,
    mut targets: Query<&mut Interactable>,
    timeline: Res<LocalTimeline>,
) {
    let tick = timeline.tick();
    for (player, position, action) in &players {
        if !action.0.interact {
            continue; // skip if not interacting
        }
        let Some(aim) = aim::aim(&spatial, player, position.0, action.0.look, INTERACT_DISTANCE)
        else {
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
