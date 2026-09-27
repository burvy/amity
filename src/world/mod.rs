use bevy::prelude::*;

pub mod definition;
pub mod mesh;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (definition::spawn_light, mesh::init_chunk_material));
        app.add_systems(
            Update,
            (
                definition::receive_chunks,
                mesh::mark_dirty,
                mesh::start_mesh_tasks,
                mesh::finish_mesh_tasks,
            )
                .chain(),
        );
        app.add_systems(Update, definition::draw_interactables); // replicated things
        app.add_systems(Update, definition::draw_grid); // client side grid
    }
}
