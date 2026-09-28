//! Turning chunks into meshes, on background threads
//!
//! - `tasks.rs`: which chunks need a mesh, and running the mesher off the main thread
//! - `padded.rs`: the snapshot of a chunk plus its border that the mesher reads
//! - `mesher.rs`: the mesher itself: visible faces, ambient occlusion, and
//!   merging matching faces into large rectangles
//! - `textures.rs`: which atlas tile each block face shows
//! - `material.rs` and `block.wgsl`: the material every chunk shares, which
//!   repeats textures across merged rectangles
//! - `config.rs`: how strong the ambient occlusion shading is

use bevy::{asset::embedded_asset, pbr::MaterialPlugin, prelude::*};

use super::ReceiveWorld;

mod config;
pub(crate) mod material;
mod mesher;
mod padded;
pub(crate) mod tasks;
pub(crate) mod textures;

pub struct MeshingPlugin;

impl Plugin for MeshingPlugin {
    fn build(&self, app: &mut App) {
        // embedded like the textures, so native and web load it the same way
        embedded_asset!(app, "block.wgsl");
        app.add_plugins(MaterialPlugin::<material::BlockMaterial>::default());
        app.add_systems(Startup, material::init_chunk_material);
        app.add_systems(
            Update,
            (tasks::mark_dirty, tasks::start_mesh_tasks, tasks::finish_mesh_tasks)
                .chain()
                .after(ReceiveWorld),
        );
    }
}
