//! The client's copy of the voxel world: chunks and edits from the server,
//! our edit requests, and drawing it all

use bevy::prelude::*;
use lightyear::core::timeline::is_in_rollback;

pub(crate) mod chunks;
mod edit;
pub(crate) mod meshing;

/// The systems that apply what the server sent. Meshing runs after these
#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ReceiveWorld;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(meshing::MeshingPlugin);
        app.init_resource::<chunks::IncomingChunks>();
        app.init_resource::<edit::HeldChanges>();
        // unloads before chunks, so a chunk dropped and resent in one frame ends up
        // loaded; block changes last, so they land on chunks spawned this frame
        app.add_systems(
            Update,
            (
                chunks::receive_unloads,
                chunks::receive_chunks,
                chunks::spawn_ready_chunks,
                edit::receive_block_changes,
            )
                .chain()
                .in_set(ReceiveWorld),
        );
        // rollback replays FixedUpdate ticks; requesting again would repeat the edit
        app.add_systems(
            FixedUpdate,
            edit::request_edits.run_if(not(is_in_rollback)),
        );
    }
}
