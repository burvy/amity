use bevy::prelude::*;

pub mod definition;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, definition::spawn_light);
        app.add_systems(Update, definition::receive_chunks);
        app.add_systems(Update, definition::draw_interactables); // replicated things
        app.add_systems(Update, definition::draw_grid); // client side grid
    }
}
