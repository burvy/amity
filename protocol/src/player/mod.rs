//! Players: their inputs, body, movement and aim, run identically on client
//! and server

use bevy::prelude::*;
use lightyear::{
    input::{config::InputConfig, native::plugin::InputPlugin},
    prelude::AppComponentExt,
};

pub mod aim;
pub mod config;
pub mod definition;
pub mod movement;
pub mod step;

pub struct PlayerProtocolPlugin;

impl Plugin for PlayerProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<definition::PlayerMarker>().replicate();

        app.add_plugins(InputPlugin::<definition::PlayerInputs> {
            config: InputConfig {
                rebroadcast_inputs: true,
                ..default()
            },
        });
    }
}
