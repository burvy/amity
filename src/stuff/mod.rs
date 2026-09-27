//! Drawing things in the world players can use

use bevy::prelude::*;

mod logic;

pub struct StuffPlugin;

impl Plugin for StuffPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, logic::draw_interactables); // replicated things
    }
}
