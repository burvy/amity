//! Deciding which chunks need a new mesh, and building meshes off the main thread

use std::{collections::HashSet, iter};

use amity_protocol::world::{
    block::Block,
    blocks::BlockSet,
    chunk::{Chunk, ChunkIndex, ChunkPos},
    config::CHUNK_SIZE,
};
use bevy::{
    prelude::*,
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};

use super::{material::ChunkMaterial, mesher::{chunk_mesh, FACES}, padded::PaddedChunk};

/// Marks a chunk for (re)meshing
#[derive(Component)]
pub struct NeedsMesh;

/// A mesh being built off the main thread. Replacing or removing this drops the
/// task, which cancels it, so an outdated mesh can never land.
#[derive(Component)]
pub struct MeshTask(Task<Mesh>);

/// A chunk is meshed once it and all six face neighbours are loaded: its border
/// faces depend on them, so meshing sooner would be redone as each one arrives.
/// A new chunk can complete itself or any of its neighbours. An edited block
/// remeshes its chunk, plus a neighbour when the block is on their shared face.
/// Chunks at the edge of the loaded area are never meshed; that ring is
/// what the server's view distance leaves out.
pub fn mark_dirty(
    mut cmds: Commands,
    added: Query<&ChunkPos, Added<Chunk>>,
    positions: Query<&ChunkPos>,
    mut changes: MessageReader<BlockSet>,
    index: Res<ChunkIndex>,
) {
    let ready = |pos: IVec3| {
        index.contains(ChunkPos(pos))
            && FACES
                .iter()
                .all(|face| index.contains(ChunkPos(pos + face.normal)))
    };
    let mut dirty = HashSet::new();
    for pos in &added {
        dirty.extend(iter::once(pos.0).chain(FACES.iter().map(|face| pos.0 + face.normal)));
    }
    for change in changes.read() {
        let Ok(pos) = positions.get(change.chunk) else {
            continue;
        };
        dirty.insert(pos.0);
        for face in &FACES {
            let beyond = change.local + face.normal;
            if beyond.cmplt(IVec3::ZERO).any() || beyond.cmpge(IVec3::splat(CHUNK_SIZE)).any() {
                dirty.insert(pos.0 + face.normal);
            }
        }
    }
    for pos in dirty.into_iter().filter(|&pos| ready(pos)) {
        if let Some(entity) = index.get(ChunkPos(pos)) {
            cmds.entity(entity).insert(NeedsMesh);
        }
    }
}

/// Snapshots dirty chunks on the main thread and meshes the snapshots on the
/// async compute pool. All-air chunks have nothing to draw and skip it.
pub fn start_mesh_tasks(
    mut cmds: Commands,
    dirty: Query<(Entity, &ChunkPos, &Chunk), With<NeedsMesh>>,
    chunks: Query<&Chunk>,
    index: Res<ChunkIndex>,
) {
    let pool = AsyncComputeTaskPool::get();
    for (entity, pos, chunk) in &dirty {
        let mut entity = cmds.entity(entity);
        entity.remove::<NeedsMesh>();
        if *chunk == Chunk::Uniform(Block::AIR) {
            entity.remove::<(MeshTask, Mesh3d)>();
            continue;
        }
        let padded = PaddedChunk::new(chunk, |offset| {
            index
                .get(ChunkPos(pos.0 + offset))
                .and_then(|neighbour| chunks.get(neighbour).ok())
        });
        entity.insert(MeshTask(pool.spawn(async move { chunk_mesh(&padded) })));
    }
}

/// Swaps in finished meshes
pub fn finish_mesh_tasks(
    mut cmds: Commands,
    mut tasks: Query<(Entity, &mut MeshTask)>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<ChunkMaterial>,
) {
    for (entity, mut task) in &mut tasks {
        let Some(mesh) = check_ready(&mut task.0) else {
            continue;
        };
        let mut entity = cmds.entity(entity);
        entity.remove::<MeshTask>();
        if mesh.count_vertices() == 0 {
            entity.remove::<Mesh3d>();
        } else {
            entity.insert((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.0.clone()),
            ));
        }
    }
}
