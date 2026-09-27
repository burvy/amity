//! Turning chunks into meshes, on background threads
//!
//! - `tasks.rs`: which chunks need a mesh, and running the mesher off the main thread
//! - `padded.rs`: the snapshot of a chunk plus its border that the mesher reads
//! - `mesher.rs`: the mesher itself
//! - `textures.rs`: which atlas tile each block face shows
//! - `material.rs`: the one material every chunk shares

use bevy::prelude::*;

use super::ReceiveWorld;

pub(crate) mod material;
mod mesher;
mod padded;
mod tasks;
pub(crate) mod textures;

pub struct MeshingPlugin;

impl Plugin for MeshingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, material::init_chunk_material);
        app.add_systems(
            Update,
            (tasks::mark_dirty, tasks::start_mesh_tasks, tasks::finish_mesh_tasks)
                .chain()
                .after(ReceiveWorld),
        );
    }
}
