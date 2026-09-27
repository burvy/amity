//! Mouse and keyboard: pausing, looking, and the inputs sent to the server

use amity_protocol::{
    inventory::{
        config::HOTBAR_SLOTS,
        definition::{ActionChannel, GameMode, ToggleFlight},
    },
    player::definition::PlayerInputs,
};
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use lightyear::prelude::{
    input::native::{ActionState, InputMarker},
    Controlled, MessageSender,
};

use super::{config::*, definition::PlayerInfo};

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
    if player_info.paused && !player_info.menu_open && buttons.just_pressed(MouseButton::Left) {
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

    let held = |key: KeyCode| f32::from(keys.pressed(key));
    action.0 = PlayerInputs {
        look: look_res.look,
        motion: Vec2 {
            x: held(KEY_RIGHT) - held(KEY_LEFT),
            y: held(KEY_BACK) - held(KEY_FORWARD), // z motion is flipped in bevy
        },
        // Hold spacebar to jump repeatedly
        jump: keys.pressed(KEY_JUMP),
        sprint: keys.any_pressed(KEY_SPRINT),
        interact: keys.pressed(KEY_INTERACT),
        primary: mouse.pressed(BUTTON_PRIMARY),
        secondary: mouse.pressed(BUTTON_SECONDARY),
        descend: keys.pressed(KEY_CROUCH),
        slot: look_res.slot,
    };
}

/// Picks the hotbar slot: its number key, or the scroll wheel (wrapping round)
pub fn select_slot(
    mut info: ResMut<PlayerInfo>,
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
) {
    if info.paused {
        return;
    }
    if let Some(slot) = KEY_SLOTS.iter().position(|&key| keys.just_pressed(key)) {
        info.slot = slot as u8;
    }
    let slots = HOTBAR_SLOTS as i32;
    let step = -scroll.delta.y.signum() as i32;
    if scroll.delta.y != 0.0 {
        info.slot = (info.slot as i32 + step).rem_euclid(slots) as u8;
    }
}

/// Double-tapping jump starts or stops flying, in creative. The server decides:
/// the request goes as a reliable message, and `Flying` comes back replicated
pub fn toggle_flight(
    info: Res<PlayerInfo>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mode: Option<Single<&GameMode, With<Controlled>>>,
    mut sender: Single<&mut MessageSender<ToggleFlight>>,
    mut last_press: Local<Option<f32>>,
) {
    if info.paused || !keys.just_pressed(KEY_JUMP) {
        return;
    }
    let now = time.elapsed_secs();
    let double = last_press.is_some_and(|last| now - last <= DOUBLE_TAP_SECS);
    // a double-tap uses both presses up; a third starts a new one
    *last_press = if double { None } else { Some(now) };
    if double && mode.is_some_and(|mode| **mode == GameMode::Creative) {
        sender.send::<ActionChannel>(ToggleFlight);
    }
}
