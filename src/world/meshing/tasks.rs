//! Deciding which chunks need a new mesh, and building meshes off the main thread

use std::{collections::HashSet, iter};

use amity_protocol::world::{
    block::Block,
    blocks::BlockSet,
    chunk::{Chunk, ChunkIndex, ChunkPos, ChunkSky},
    config::CHUNK_SIZE,
};
use bevy::{
    light::NotShadowCaster,
    prelude::*,
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};

use crate::world::sky::SkyChanged;

use super::{
    material::ChunkMaterials,
    mesher::{chunk_mesh, ChunkMeshes},
    config::SKY_LIGHT_MAX,
    padded::{neighbour_offsets, Neighbourhood, PaddedChunk},
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

/// A chunk is meshed once it and all 26 chunks around it are loaded: its
/// border faces, and the shading at its edges (ambient occlusion looks
/// diagonally), depend on them, so meshing sooner would be redone as each one
/// arrives. A new chunk can complete itself or any chunk around it. An edited
/// block remeshes its chunk, plus every chunk its effect reaches: sky light
/// spreads up to SKY_LIGHT_MAX blocks, so a block that close to a chunk's edge
/// changes the chunk across it too (up to 7 more, near a corner). A chunk whose
/// sky changed (`world/sky.rs`) remeshes with every chunk around it. Chunks at
/// the edge of the loaded area are never meshed; that ring is what the
/// server's view distance leaves out.
pub fn mark_dirty(
    mut cmds: Commands,
    added: Query<&ChunkPos, Added<Chunk>>,
    positions: Query<&ChunkPos>,
    mut changes: MessageReader<BlockSet>,
    mut skies: MessageReader<SkyChanged>,
    index: Res<ChunkIndex>,
) {
    let ready = |pos: IVec3| {
        index.contains(ChunkPos(pos))
            && neighbour_offsets().all(|offset| index.contains(ChunkPos(pos + offset)))
    };
    let mut dirty = HashSet::new();
    for pos in &added {
        dirty.extend(iter::once(pos.0).chain(neighbour_offsets().map(|offset| pos.0 + offset)));
    }
    for change in changes.read() {
        let Ok(pos) = positions.get(change.chunk) else {
            continue;
        };
        dirty.insert(pos.0);
        // the chunks it reaches: on each axis, the one beyond it if the block
        // is within light's reach of that edge
        let reach = SKY_LIGHT_MAX as i32;
        for offset in neighbour_offsets() {
            let reaches = (0..3).all(|axis| match offset[axis] {
                -1 => change.local[axis] < reach,
                1 => change.local[axis] >= CHUNK_SIZE - reach,
                _ => true,
            });
            if reaches {
                dirty.insert(pos.0 + offset);
            }
        }
    }
    for SkyChanged(pos) in skies.read() {
        dirty.extend(iter::once(*pos).chain(neighbour_offsets().map(|offset| *pos + offset)));
    }
    for pos in dirty.into_iter().filter(|&pos| ready(pos)) {
        if let Some(entity) = index.get(ChunkPos(pos)) {
            cmds.entity(entity).insert(NeedsMesh);
        }
    }
}

/// Copies dirty chunks and the chunks around them on the main thread, and
/// lights and meshes the copies on the async compute pool. All-air chunks have
/// nothing to draw and skip it.
pub fn start_mesh_tasks(
    mut cmds: Commands,
    dirty: Query<(Entity, &ChunkPos, &Chunk, Option<&FluidMesh>), With<NeedsMesh>>,
    chunks: Query<(&Chunk, Option<&ChunkSky>)>,
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
        let area = Neighbourhood::gather(|offset| {
            index
                .get(ChunkPos(pos.0 + offset))
                .and_then(|chunk| chunks.get(chunk).ok())
        });
        entity.insert(MeshTask(pool.spawn(async move {
            chunk_mesh(&PaddedChunk::new(&area))
        })));
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
