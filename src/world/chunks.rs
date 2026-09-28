//! Chunks arriving from and leaving at the server's request

use amity_protocol::world::{
    chunk::{chunk_body, collider::chunk_collider, Chunk, ChunkIndex, ChunkPos},
    messages::{ChunkData, UnloadChunk},
};
use avian3d::prelude::Collider;
use bevy::{
    platform::collections::HashMap,
    prelude::*,
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};
use lightyear::prelude::MessageReceiver;

/// Chunks that arrived and are having their collider built on a background
/// thread, by position. Building a collider for a chunk of mixed blocks takes a
/// few milliseconds; for the dozens that can arrive at once, that stalled whole
/// frames on the main thread. A chunk is only spawned once its collider is ready,
/// so players never stand on a chunk that can't hold them yet
#[derive(Resource, Default)]
pub struct IncomingChunks(HashMap<IVec3, Task<(Chunk, Option<Collider>)>>);

impl IncomingChunks {
    /// Whether a (newer) copy of this chunk is on its way in
    pub fn contains(&self, pos: ChunkPos) -> bool {
        self.0.contains_key(&pos.0)
    }

    /// How many chunks are still having their collider built
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// Starts building colliders for chunks sent by the server. A chunk that's
/// already on its way (or loaded) is replaced by the newer copy
pub fn receive_chunks(
    mut receiver: Single<&mut MessageReceiver<ChunkData>>,
    mut incoming: ResMut<IncomingChunks>,
) {
    let pool = AsyncComputeTaskPool::get();
    for ChunkData { pos, chunk } in receiver.receive() {
        // replacing a task drops the old one, which cancels it
        let task = pool.spawn(async move {
            let collider = chunk_collider(&chunk);
            (chunk, collider)
        });
        incoming.0.insert(pos.0, task);
    }
}

/// Spawns chunks whose collider is ready, replacing any old copy
pub fn spawn_ready_chunks(
    mut cmds: Commands,
    mut incoming: ResMut<IncomingChunks>,
    index: Res<ChunkIndex>,
) {
    incoming.0.retain(|&pos, task| {
        let Some((chunk, collider)) = check_ready(task) else {
            return true;
        };
        let pos = ChunkPos(pos);
        if let Some(old) = index.get(pos) {
            cmds.entity(old).despawn();
        }
        // Visibility: a chunk's water is drawn by a child entity, and bevy only
        // shows children of entities that have one. Chunks with no solid mesh
        // (open sea) wouldn't get one otherwise, and their water would be invisible
        let mut entity = cmds.spawn((chunk_body(pos, chunk), Visibility::default()));
        if let Some(collider) = collider {
            entity.insert(collider);
        }
        false
    });
}

/// Despawns chunks the server says are out of range, and cancels any still on
/// their way in. The mesh and collider go with the entity
pub fn receive_unloads(
    mut cmds: Commands,
    mut receiver: Single<&mut MessageReceiver<UnloadChunk>>,
    mut incoming: ResMut<IncomingChunks>,
    index: Res<ChunkIndex>,
) {
    for UnloadChunk { pos } in receiver.receive() {
        incoming.0.remove(&pos.0);
        if let Some(entity) = index.get(pos) {
            cmds.entity(entity).despawn();
        }
    }
}
