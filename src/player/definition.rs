use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

/// Mouse sensitivity constant
/// Can be kept client side for now since it should be configurable later
/// TODO: Make configurable
const SENSITIVITY: f32 = 0.0025;

/// Resource to store client-side player info
#[derive(Resource, Default)]
pub struct PlayerInfo {
    pub paused: bool,
    pub look: Vec2,
    pub can_interact: bool,
}

impl PlayerInfo {
    /// Updates the current yaw and pitch based on mouse motion
    /// accumulation
    pub fn update_look(&mut self, accumulation: Vec2) {
        self.look.x -= accumulation.x * SENSITIVITY;
        self.look.y -= accumulation.y * SENSITIVITY;
        self.look.y = self.look.y.clamp(-FRAC_PI_2, FRAC_PI_2);
    }
}

#[derive(Component, Clone, Default)]
pub struct MainPlayerCamera;

pub fn spawn_camera(mut cmds: Commands) {
    cmds.spawn_scene(bsn! {
        #Camera
        Camera3d
        MainPlayerCamera
    });
}

#[derive(Component)]
pub struct InteractionPrompt;

pub fn spawn_interact_prompt(mut cmds: Commands) {
    cmds.spawn((
        InteractionPrompt,
        Text::new(format!("Press {} to interact", "E")), // TODO: configurable inputs
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(65.0),
            width: Val::Percent(100.0),
            ..default()
        },
        TextLayout::justify(Justify::Center),
        Visibility::Hidden,
    ));
}
