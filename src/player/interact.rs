//! The "Press E to interact" prompt

use amity_protocol::{
    player::{aim, config::INTERACT_DISTANCE},
    stuff::definition::Interactable,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::*;

use super::definition::{InteractionPrompt, PlayerInfo};

/// Sets can_interact to true on PlayerInfo if lookng at an entity that contains Interactable
pub fn client_interaction_checker(
    spatial: SpatialQuery,
    mut player_info: ResMut<PlayerInfo>,
    player: Single<(Entity, &Position), With<Controlled>>,
    interactables: Query<&Interactable>,
    timeline: Res<LocalTimeline>,
) {
    let (player, position) = player.into_inner();

    let tick = timeline.tick();
    let should_interact = aim::aim(
        &spatial,
        player,
        position.0,
        player_info.look,
        INTERACT_DISTANCE,
    )
    .is_some_and(|aim| interactables.get(aim.entity).is_ok_and(|i| i.ready(tick)));

    if player_info.can_interact != should_interact {
        player_info.can_interact = should_interact
    }
}

pub fn can_interact_prompt(
    mut prompt_visibility: Single<&mut Visibility, With<InteractionPrompt>>,
    player_info: Res<PlayerInfo>,
) {
    let target = if player_info.can_interact {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    if **prompt_visibility != target {
        **prompt_visibility = target;
    }
}
