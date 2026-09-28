//! Deciding which chunks need a new mesh, and building meshes off the main thread

use std::{collections::HashSet, iter};

use amity_protocol::world::{
    block::Block,
    blocks::BlockSet,
    chunk::{Chunk, ChunkIndex, ChunkPos},
    config::CHUNK_SIZE,
};
use bevy::{
    light::NotShadowCaster,
    prelude::*,
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};

use super::{
    material::ChunkMaterials,
    mesher::{chunk_mesh, ChunkMeshes, FACES},
    padded::PaddedChunk,
};

/// Marks a chunk for (re)meshing
#[derive(Component)]
pub struct NeedsMesh;

/// A mesh being built off the main thread. Replacing or removing this drops the
/// task, which cancels it, so an outdated mesh can never land.
#[derive(Component)]
pub struct MeshTask(Task<ChunkMeshes>);

/// The child entity drawing a chunk's fluids. An entity has one mesh and one
/// material, so the chunk draws its solid blocks and this child its water.
/// Despawned with the chunk
#[derive(Component)]
pub struct FluidMesh(Entity);

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
    dirty: Query<(Entity, &ChunkPos, &Chunk, Option<&FluidMesh>), With<NeedsMesh>>,
    chunks: Query<&Chunk>,
    index: Res<ChunkIndex>,
) {
    let pool = AsyncComputeTaskPool::get();
    for (entity, pos, chunk, fluid) in &dirty {
        if *chunk == Chunk::Uniform(Block::AIR) {
            if let Some(FluidMesh(child)) = fluid {
                cmds.entity(*child).despawn();
            }
            cmds.entity(entity)
                .remove::<(NeedsMesh, MeshTask, Mesh3d, FluidMesh)>();
            continue;
        }
        let mut entity = cmds.entity(entity);
        entity.remove::<NeedsMesh>();
        let padded = PaddedChunk::new(chunk, |offset| {
            index
                .get(ChunkPos(pos.0 + offset))
                .and_then(|neighbour| chunks.get(neighbour).ok())
        });
        entity.insert(MeshTask(pool.spawn(async move { chunk_mesh(&padded) })));
    }
}

/// Swaps in finished meshes: the solid one on the chunk itself, the fluid one on
/// its `FluidMesh` child, which is spawned the first time a chunk has water and
/// despawned when it no longer does
pub fn finish_mesh_tasks(
    mut cmds: Commands,
    mut tasks: Query<(Entity, &mut MeshTask, Option<&FluidMesh>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<ChunkMaterials>,
) {
    for (chunk, mut task, fluid_child) in &mut tasks {
        let Some(ChunkMeshes { solid, fluid }) = check_ready(&mut task.0) else {
            continue;
        };
        let mut entity = cmds.entity(chunk);
        entity.remove::<MeshTask>();
        if solid.count_vertices() == 0 {
            entity.remove::<Mesh3d>();
        } else {
            entity.insert((
                Mesh3d(meshes.add(solid)),
                MeshMaterial3d(materials.solid.clone()),
            ));
        }

        match (fluid.count_vertices() == 0, fluid_child) {
            // no water, no child: nothing to do
            (true, None) => {}
            // the water is gone
            (true, Some(FluidMesh(child))) => {
                cmds.entity(*child).despawn();
                cmds.entity(chunk).remove::<FluidMesh>();
            }
            // water changed: swap the child's mesh
            (false, Some(FluidMesh(child))) => {
                cmds.entity(*child).insert(Mesh3d(meshes.add(fluid)));
            }
            // first water in this chunk: a child at the chunk's origin to draw it
            (false, None) => {
                let child = cmds
                    .spawn((
                        Mesh3d(meshes.add(fluid)),
                        MeshMaterial3d(materials.fluid.clone()),
                        // water lets light through: a shadow from it would be
                        // wrong, and costs a draw in every shadow pass
                        NotShadowCaster,
                        ChildOf(chunk),
                    ))
                    .id();
                cmds.entity(chunk).insert(FluidMesh(child));
            }
        }
    }
}
