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

/// The child entities drawing a chunk's see-through blocks and fluids. An
/// entity has one mesh and one material, so the chunk draws its solid blocks
/// and these children the rest. Each is spawned the first time the chunk has
/// something for it to draw and despawned when it no longer does, so most
/// chunks have neither. Despawned with the chunk
#[derive(Component, Default, PartialEq)]
pub struct ChunkParts {
    cutout: Option<Entity>,
    fluid: Option<Entity>,
}

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
    dirty: Query<(Entity, &ChunkPos, &Chunk, Option<&ChunkParts>), With<NeedsMesh>>,
    chunks: Query<(&Chunk, Option<&ChunkSky>)>,
    index: Res<ChunkIndex>,
) {
    let pool = AsyncComputeTaskPool::get();
    for (entity, pos, chunk, parts) in &dirty {
        if *chunk == Chunk::Uniform(Block::AIR) {
            for child in parts.into_iter().flat_map(|parts| [parts.cutout, parts.fluid]).flatten() {
                cmds.entity(child).despawn();
            }
            cmds.entity(entity)
                .remove::<(NeedsMesh, MeshTask, Mesh3d, ChunkParts)>();
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

/// Swaps in finished meshes: the solid one on the chunk itself, the
/// see-through and fluid ones on its `ChunkParts` children
pub fn finish_mesh_tasks(
    mut cmds: Commands,
    mut tasks: Query<(Entity, &mut MeshTask, Option<&mut ChunkParts>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<ChunkMaterials>,
) {
    for (chunk, mut task, parts) in &mut tasks {
        let Some(ChunkMeshes { solid, cutout, fluid }) = check_ready(&mut task.0) else {
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

        let (cutout_child, fluid_child) = parts
            .as_deref()
            .map_or((None, None), |parts| (parts.cutout, parts.fluid));
        let now = ChunkParts {
            // leaves cast (dappled) shadows
            cutout: swap_part(&mut cmds, &mut meshes, chunk, cutout_child, cutout, || {
                MeshMaterial3d(materials.cutout.clone())
            }),
            // water lets light through: a shadow from it would be wrong, and
            // costs a draw in every shadow pass
            fluid: swap_part(&mut cmds, &mut meshes, chunk, fluid_child, fluid, || {
                (MeshMaterial3d(materials.fluid.clone()), NotShadowCaster)
            }),
        };
        match parts {
            Some(mut parts) => {
                parts.set_if_neq(now);
            }
            None if now != ChunkParts::default() => {
                cmds.entity(chunk).insert(now);
            }
            None => {}
        }
    }
}

/// Puts `mesh` on one of a chunk's children: spawning it (with `extra`) if
/// the chunk has none, despawning it if the mesh is empty. Returns the child
/// the chunk has now
fn swap_part<B: Bundle>(
    cmds: &mut Commands,
    meshes: &mut Assets<Mesh>,
    chunk: Entity,
    child: Option<Entity>,
    mesh: Mesh,
    extra: impl FnOnce() -> B,
) -> Option<Entity> {
    match (mesh.count_vertices() == 0, child) {
        (true, None) => None,
        (true, Some(child)) => {
            cmds.entity(child).despawn();
            None
        }
        (false, Some(child)) => {
            cmds.entity(child).insert(Mesh3d(meshes.add(mesh)));
            Some(child)
        }
        (false, None) => Some(
            cmds.spawn((Mesh3d(meshes.add(mesh)), extra(), ChildOf(chunk)))
                .id(),
        ),
    }
}
