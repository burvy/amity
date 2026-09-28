//! The flashlight: a beam from every player's eye, pointing where they look.
//! It comes on by itself when night falls and goes off at sunrise; the
//! Flashlight key (F) switches it in between. Whether ours is on rides in our
//! inputs, which lightyear already shares with every player, so everyone sees
//! everyone's beam.
//!
//! Only our own beam casts shadows: a spot light's shadows are drawn again
//! every frame, and a crowd of them would cost more than they add.

use amity_protocol::{
    player::{config::EYE_HEIGHT, definition::PlayerInputs},
    sky::astronomy::sun_direction,
};
use bevy::prelude::*;
use lightyear::prelude::{input::native::ActionState, Controlled};

use super::{
    config::{
        FLASHLIGHT_HAND, FLASHLIGHT_INNER_ANGLE, FLASHLIGHT_LUMENS, FLASHLIGHT_NIGHT_SUN_DEGREES,
        FLASHLIGHT_OUTER_ANGLE, FLASHLIGHT_RANGE,
    },
    definition::PlayerInfo,
    glide::StepGlide,
};
use crate::{
    settings::{definition::Action, input::ActionInput},
    sky::logic::SkyTime,
};

/// Our flashlight's switch
#[derive(Resource, Default)]
pub struct FlashlightSwitch {
    pub on: bool,
    /// Whether it was night last time we looked (None before the sky's time
    /// arrives). The flashlight follows it only when it changes, so switching
    /// it by hand sticks until the next dusk or dawn
    night: Option<bool>,
}

/// A player's beam: a child of their body
#[derive(Component)]
pub struct Beam;

/// The beam for a player's body, off until they switch it on
pub fn beam() -> impl Bundle {
    (
        Beam,
        SpotLight {
            intensity: FLASHLIGHT_LUMENS,
            range: FLASHLIGHT_RANGE,
            outer_angle: FLASHLIGHT_OUTER_ANGLE,
            inner_angle: FLASHLIGHT_INNER_ANGLE,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
        Visibility::Hidden,
    )
}

/// Switches our flashlight on at dusk and off at dawn, and by hand with the
/// Flashlight key
pub fn switch_flashlight(
    mut switch: ResMut<FlashlightSwitch>,
    sky: Res<SkyTime>,
    input: ActionInput,
    info: Res<PlayerInfo>,
) {
    if let Some(days) = sky.days() {
        let night = sun_direction(days).y < FLASHLIGHT_NIGHT_SUN_DEGREES.to_radians().sin();
        if switch.night != Some(night) {
            switch.night = Some(night);
            switch.on = night;
        }
    }
    if !info.paused && !info.menu_open && input.just_pressed(Action::Flashlight) {
        switch.on = !switch.on;
    }
}

/// Points every player's beam where they look, from their hand, and shows it
/// if their flashlight is on. Ours follows our own look and switch straight
/// away; others' follow the inputs they sent. Runs before transforms are
/// handed to rendering, with the camera, so the beam never lags the view
pub fn aim_beams(
    mut beams: Query<(&ChildOf, &mut Transform, &mut Visibility, &mut SpotLight), With<Beam>>,
    players: Query<(&ActionState<PlayerInputs>, &StepGlide, Has<Controlled>)>,
    info: Res<PlayerInfo>,
    switch: Res<FlashlightSwitch>,
) {
    for (parent, mut transform, mut visibility, mut light) in &mut beams {
        let Ok((action, glide, ours)) = players.get(parent.parent()) else {
            continue;
        };
        let (look, on) = if ours {
            (info.look, switch.on)
        } else {
            (action.0.look, action.0.flashlight)
        };
        visibility.set_if_neq(if on { Visibility::Inherited } else { Visibility::Hidden });
        if light.shadow_maps_enabled != ours {
            light.shadow_maps_enabled = ours;
        }
        if !on {
            continue;
        }
        let aim = Quat::from_euler(EulerRot::YXZ, look.x, look.y, 0.0);
        let eye = Vec3::Y * (EYE_HEIGHT + glide.offset);
        *transform = Transform::from_translation(eye + aim * FLASHLIGHT_HAND).with_rotation(aim);
    }
}
