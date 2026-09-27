use bevy::prelude::*;
use lightyear::core::timeline::is_in_rollback;

pub mod definition;
pub mod edit;
pub mod mesh;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (definition::spawn_light, mesh::init_chunk_material),
        );
        app.add_systems(
            Update,
            (
                definition::receive_chunks,
                edit::receive_block_changes,
                mesh::mark_dirty,
                mesh::start_mesh_tasks,
                mesh::finish_mesh_tasks,
            )
                .chain(),
        );
        // rollback replays FixedUpdate ticks; requesting again would repeat the edit
        app.add_systems(
            FixedUpdate,
            edit::request_edits.run_if(not(is_in_rollback)),
        );
        app.add_systems(Update, definition::draw_interactables); // replicated things
    }
}
