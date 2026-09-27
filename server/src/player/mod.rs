//! Spawning players and simulating them on the server

use amity_protocol::player::{
    definition::{PlayerInputs, PlayerMarker},
    movement, step,
};
use bevy::prelude::*;
use lightyear::input::server::{authorize_controlled_targets, InputValidationAppExt};
use lightyear::prelude::input::native::NativeStateSequence;

pub mod config;
mod logic;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(logic::spawn_player);

        app.add_systems(
            FixedUpdate,
            (
                logic::clear_stale_input,
                movement::player_motion::<With<PlayerMarker>>,
                step::step_up::<With<PlayerMarker>>,
                logic::keep_players_replicating,
            )
                .chain(),
        );

        // this is not default, but prevents clients from controlling
        // entities they shouldn't be on the server
        app.add_input_validator(authorize_controlled_targets::<NativeStateSequence<PlayerInputs>>);
    }
}
