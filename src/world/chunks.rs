//! Chunks arriving from and leaving at the server's request

use amity_protocol::world::{
    chunk::{chunk_body, ChunkIndex},
    messages::{ChunkData, UnloadChunk},
};
use bevy::prelude::*;
use lightyear::prelude::MessageReceiver;

/// Spawns chunks sent by the server. A chunk sent again replaces the old copy
pub fn receive_chunks(
    mut cmds: Commands,
    mut receiver: Single<&mut MessageReceiver<ChunkData>>,
    index: Res<ChunkIndex>,
) {
    for ChunkData { pos, chunk } in receiver.receive() {
        if let Some(old) = index.get(pos) {
            cmds.entity(old).despawn();
        }
        // Visibility: a chunk's water is drawn by a child entity, and bevy only
        // shows children of entities that have one. Chunks with no solid mesh
        // (open sea) wouldn't get one otherwise, and their water would be invisible
        cmds.spawn((chunk_body(pos, chunk), Visibility::default()));
    }
}

/// Despawns chunks the server says are out of range. The mesh and collider go
/// with the entity
pub fn receive_unloads(
    mut cmds: Commands,
    mut receiver: Single<&mut MessageReceiver<UnloadChunk>>,
    index: Res<ChunkIndex>,
) {
    for UnloadChunk { pos } in receiver.receive() {
        if let Some(entity) = index.get(pos) {
            cmds.entity(entity).despawn();
        }
    }
}
