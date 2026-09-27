//! Mouse and keyboard: pausing, looking, and the inputs sent to the server

use amity_protocol::player::definition::PlayerInputs;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use lightyear::prelude::input::native::{ActionState, InputMarker};

use super::definition::PlayerInfo;

/// Unlock on escape
pub fn pause_on_esc(
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut player_info: ResMut<PlayerInfo>,
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
    mut player_info: ResMut<PlayerInfo>,
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
    mut look_res: ResMut<PlayerInfo>,
    mouse_mot_res: Res<AccumulatedMouseMotion>,
) {
    if !look_res.paused {
        look_res.update_look(mouse_mot_res.delta);
    }
}

/// Player input for KEYBOARD for now!
/// TODO: allow input for other devices (mobile?)
/// TODO: allow configurable input controls
pub fn buffer_input(
    mut action: Single<&mut ActionState<PlayerInputs>, With<InputMarker<PlayerInputs>>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    look_res: Res<PlayerInfo>, // yaw/pitch stored in resource
) {
    if look_res.paused {
        action.0 = PlayerInputs {
            look: look_res.look,
            ..default()
        };
        return;
    }

    // TODO: configurable inputs
    let fwd = f32::from(keys.pressed(KeyCode::KeyW));
    let bwd = f32::from(keys.pressed(KeyCode::KeyS));
    let rwd = f32::from(keys.pressed(KeyCode::KeyD));
    let lwd = f32::from(keys.pressed(KeyCode::KeyA));

    action.0 = PlayerInputs {
        look: look_res.look,
        motion: Vec2 {
            x: rwd - lwd,
            y: bwd - fwd, // z motion is flipped in bevy
        },
        // Hold spacebar to jump repeatedly
        jump: keys.pressed(KeyCode::Space), // TODO: configurable inputs
        sprint: keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight), // TODO: configurable inputs
        interact: keys.pressed(KeyCode::KeyE), // TODO: configurable inputs
        primary: mouse.pressed(MouseButton::Left), // TODO: configurable inputs
        secondary: mouse.pressed(MouseButton::Right), // TODO: configurable inputs
    };
}
