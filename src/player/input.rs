//! Mouse and keyboard: looking, and the inputs sent to the server. Pausing is
//! the pause menu's (`ui/pause.rs`)

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
};
use lightyear::prelude::{
    input::native::{ActionState, InputMarker},
    Controlled, MessageSender,
};

use super::{config::*, definition::PlayerInfo, flashlight::FlashlightSwitch};
use crate::settings::{
    definition::{Action, Settings},
    input::ActionInput,
};

/// Stores the accumulated mouse motion in the PlayerInfo resource
pub fn rotate_player_resource(
    mut look_res: ResMut<PlayerInfo>,
    mouse_mot_res: Res<AccumulatedMouseMotion>,
    settings: Res<Settings>,
) {
    // the mouse moves the cursor instead while the inventory is open
    if !look_res.paused && !look_res.menu_open {
        let controls = &settings.controls;
        look_res.update_look(mouse_mot_res.delta, controls.sensitivity, controls.invert_y);
    }
}

/// Player input for KEYBOARD for now!
/// TODO: allow input for other devices (mobile?)
/// TODO: allow configurable input controls
pub fn buffer_input(
    mut action: Single<&mut ActionState<PlayerInputs>, With<InputMarker<PlayerInputs>>>,
    input: ActionInput,
    look_res: Res<PlayerInfo>, // yaw/pitch stored in resource
    flashlight: Res<FlashlightSwitch>,
) {
    if look_res.paused {
        action.0 = PlayerInputs {
            look: look_res.look,
            // the flashlight stays as it is while paused
            flashlight: flashlight.on,
            ..default()
        };
        return;
    }

    let held = |action| f32::from(input.pressed(action));
    action.0 = PlayerInputs {
        look: look_res.look,
        motion: Vec2 {
            x: held(Action::Right) - held(Action::Left),
            y: held(Action::Back) - held(Action::Forward), // z motion is flipped in bevy
        },
        // Hold jump to jump repeatedly
        jump: input.pressed(Action::Jump),
        sprint: input.pressed(Action::Sprint),
        interact: input.pressed(Action::Interact),
        // clicks go to the inventory while it's open, not to the world
        primary: input.pressed(Action::Break) && !look_res.menu_open,
        secondary: input.pressed(Action::Place) && !look_res.menu_open,
        descend: input.pressed(Action::Crouch),
        slot: look_res.slot,
        flashlight: flashlight.on,
    };
}

/// Picks the hotbar slot: its number key, or the scroll wheel (wrapping round)
pub fn select_slot(
    mut info: ResMut<PlayerInfo>,
    input: ActionInput,
    scroll: Res<AccumulatedMouseScroll>,
) {
    if info.paused {
        return;
    }
    if let Some(slot) = Action::SLOTS.iter().position(|&slot| input.just_pressed(slot)) {
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
    input: ActionInput,
    time: Res<Time>,
    mode: Option<Single<&GameMode, With<Controlled>>>,
    mut sender: Single<&mut MessageSender<ToggleFlight>>,
    mut last_press: Local<Option<f32>>,
) {
    if info.paused || !input.just_pressed(Action::Jump) {
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
