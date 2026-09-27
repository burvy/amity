//! Things in the world players can use

use amity_protocol::player::{definition::PlayerMarker, step};
use bevy::prelude::*;

mod logic;

pub struct StuffPlugin;

impl Plugin for StuffPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, logic::spawn_box);
        // after movement, so players interact from where they ended up this tick
        app.add_systems(
            FixedUpdate,
            logic::server_interact.after(step::step_up::<With<PlayerMarker>>),
        );
    }
}
