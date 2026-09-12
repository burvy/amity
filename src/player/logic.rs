use avian3d::prelude::*;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use lightyear::prelude::*;
use shooter_protocol::{shared, stuff};

use crate::player::definition;

/// Unlock on escape
pub fn pause_on_esc(
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut player_info: ResMut<definition::PlayerInfo>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if key.just_pressed(KeyCode::Escape) && !player_info.paused {
        player_info.paused = true;
        cursor_options.grab_mode = CursorGrabMode::None;
        cursor_options.visible = true;
    }
}

/// Lock on left click
pub fn grab_on_click(
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut player_info: ResMut<definition::PlayerInfo>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    if player_info.paused && buttons.just_pressed(MouseButton::Left) {
        player_info.paused = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
    }
}

/// Stores the accumulated mouse motion in the PlayerInfo resource
pub fn rotate_player_resource(
    mut look_res: ResMut<definition::PlayerInfo>,
    mouse_mot_res: Res<AccumulatedMouseMotion>,
) {
    if !look_res.paused {
        look_res.update_look(mouse_mot_res.delta);
    }
}

pub fn camera_follow_player(
    mut camera: Single<&mut Transform, With<definition::MainPlayerCamera>>,
    player: Single<&Position, With<Controlled>>,
    look_res: Res<definition::PlayerInfo>,
) {
    camera.translation = player.0 + Vec3::Y * shared::EYE_HEIGHT;
    camera.rotation = Quat::from_euler(EulerRot::YXZ, look_res.look.x, look_res.look.y, 0.0);
}

/// Sets can_interact to true on PlayerInfo if lookng at an entity that contains Interactable
pub fn client_interaction_checker(
    spatial: SpatialQuery,
    mut player_info: ResMut<definition::PlayerInfo>,
    player: Single<(Entity, &Position), With<Controlled>>,
    interactables: Query<(), With<stuff::definition::Interactable>>,
) {
    let (player, position) = player.into_inner();

    let rot = Quat::from_euler(EulerRot::YXZ, player_info.look.x, player_info.look.y, 0.0);
    let Ok(dir) = Dir3::new(rot * Vec3::NEG_Z) else {
        return;
    };

    let origin = position.0 + Vec3::Y * shared::EYE_HEIGHT;
    let filter = SpatialQueryFilter::from_excluded_entities([player]);

    let should_interact = spatial
        .cast_ray(origin, dir, shared::INTERACT_DISTANCE, true, &filter)
        .is_some_and(|hit| interactables.contains(hit.entity));

    if player_info.can_interact != should_interact {
        player_info.can_interact = should_interact
    }
}

pub fn can_interact_prompt() {}
