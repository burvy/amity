use std::iter;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};
use shooter_protocol::world::chunk::{Block, CHUNK_SIZE, Chunk, ChunkIndex, ChunkPos};

/// One side of a unit block. Corners are counter-clockwise seen from outside
/// (bevy's front face), in the order bottom-left, bottom-right, top-right, top-left.
/// faces are not drawn from the inside and are drawn from the outside.
struct Face {
    normal: IVec3,
    corners: [[f32; 3]; 4],
}

const FACES: [Face; 6] = [
    Face {
        normal: IVec3::X,
        corners: [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]],
    },
    Face {
        normal: IVec3::NEG_X,
        corners: [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]],
    },
    Face {
        normal: IVec3::Y,
        corners: [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]],
    },
    Face {
        normal: IVec3::NEG_Y,
        corners: [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
    },
    Face {
        normal: IVec3::Z,
        corners: [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
    },
    Face {
        normal: IVec3::NEG_Z,
        corners: [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
    },
];

/// Matches `Face::corners`. Every face gets the whole 0..1 square, so a texture
/// atlas only has to remap these into the block's tile.
const FACE_UVS: [[f32; 2]; 4] = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];

/// Shared by every chunk. Vertex colors tint it until there is a texture atlas.
#[derive(Resource)]
pub struct ChunkMaterial(pub Handle<StandardMaterial>);

pub fn init_chunk_material(mut cmds: Commands, mut mats: ResMut<Assets<StandardMaterial>>) {
    cmds.insert_resource(ChunkMaterial(mats.add(StandardMaterial {
        perceptual_roughness: 0.25,
        ..default()
    })));
}

/// Marks a chunk for (re)meshing
#[derive(Component)]
pub struct NeedsMesh;

/// A mesh being built off the main thread. Replacing or removing this drops the
/// task, which cancels it, so an outdated mesh can never land.
#[derive(Component)]
pub struct MeshTask(Task<Mesh>);

/// A changed chunk needs a new mesh, and so do its face neighbours: their border
/// faces may now be hidden or exposed.
pub fn mark_dirty(
    mut cmds: Commands,
    changed: Query<&ChunkPos, Changed<Chunk>>,
    index: Res<ChunkIndex>,
) {
    for pos in &changed {
        for offset in iter::once(IVec3::ZERO).chain(FACES.iter().map(|face| face.normal)) {
            if let Some(entity) = index.get(ChunkPos(pos.0 + offset)) {
                cmds.entity(entity).insert(NeedsMesh);
            }
        }
    }
}

/// Snapshots dirty chunks on the main thread and meshes the snapshots on the
/// async compute pool
pub fn start_mesh_tasks(
    mut cmds: Commands,
    dirty: Query<(Entity, &ChunkPos, &Chunk), With<NeedsMesh>>,
    chunks: Query<&Chunk>,
    index: Res<ChunkIndex>,
) {
    let pool = AsyncComputeTaskPool::get();
    for (entity, pos, chunk) in &dirty {
        let padded = PaddedChunk::new(chunk, |offset| {
            index
                .get(ChunkPos(pos.0 + offset))
                .and_then(|neighbour| chunks.get(neighbour).ok())
        });
        let task = pool.spawn(async move { chunk_mesh(&padded) });
        cmds.entity(entity)
            .remove::<NeedsMesh>()
            .insert(MeshTask(task));
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

const PADDED_SIZE: i32 = CHUNK_SIZE + 2;

/// A chunk plus a one-block border copied from its six face neighbours, like
/// Minecraft's section copy. It is owned, so meshing can run on another thread
/// while the live world keeps changing, and every neighbour check is one array read.
pub struct PaddedChunk {
    blocks: Box<[Block]>,
}

impl PaddedChunk {
    /// `neighbour` returns the chunk at a chunk offset such as `IVec3::X`, if it is
    /// loaded. Missing neighbours count as air, so their border faces are drawn
    /// until they arrive and trigger a remesh.
    pub fn new<'a>(center: &Chunk, neighbour: impl Fn(IVec3) -> Option<&'a Chunk>) -> Self {
        let mut blocks = vec![Block::Air; (PADDED_SIZE * PADDED_SIZE * PADDED_SIZE) as usize];
        for y in -1..=CHUNK_SIZE {
            for z in -1..=CHUNK_SIZE {
                for x in -1..=CHUNK_SIZE {
                    let p = IVec3::new(x, y, z);
                    let (offset, local) = ChunkPos::of_block(p);
                    blocks[Self::index(p)] = match offset.0 {
                        IVec3::ZERO => center.get(local),
                        // edges and corners stay air: face culling never looks diagonally
                        o if o.abs().element_sum() == 1 => {
                            neighbour(o).map_or(Block::Air, |chunk| chunk.get(local))
                        }
                        _ => Block::Air,
                    };
                }
            }
        }
        Self {
            blocks: blocks.into_boxed_slice(),
        }
    }

    /// same layout as `Chunk`, shifted by one so -1..=CHUNK_SIZE fits
    fn index(p: IVec3) -> usize {
        let p = p + IVec3::ONE;
        (p.x + p.z * PADDED_SIZE + p.y * PADDED_SIZE * PADDED_SIZE) as usize
    }

    /// `p` is in the center chunk's local space, -1..=CHUNK_SIZE on each axis
    pub fn get(&self, p: IVec3) -> Block {
        self.blocks[Self::index(p)]
    }
}

/// Face-culled mesh: a face is emitted only where the neighbouring block is air,
/// including across chunk borders.
pub fn chunk_mesh(chunk: &PaddedChunk) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let local = IVec3::new(x, y, z);
                let block = chunk.get(local);
                if !block.is_solid() {
                    continue;
                }
                for face in &FACES {
                    if chunk.get(local + face.normal).is_solid() {
                        continue;
                    }

                    let base = positions.len() as u32;
                    let offset = local.as_vec3();
                    for corner in face.corners {
                        positions.push((offset + Vec3::from(corner)).to_array());
                    }
                    normals.extend([face.normal.as_vec3().to_array(); 4]);
                    uvs.extend(FACE_UVS);
                    colors.extend([face_color(block, face.normal); 4]);
                    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                }
            }
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

/// Per-face so grass can have a different top and sides. When textures come in,
/// this becomes the atlas tile lookup.
fn face_color(block: Block, normal: IVec3) -> [f32; 4] {
    let srgb = match (block, normal) {
        (Block::Grass, IVec3::Y) => Color::srgb_u8(95, 159, 53),
        (Block::Grass | Block::Dirt, _) => Color::srgb_u8(134, 96, 67),
        (Block::Stone, _) => Color::srgb_u8(125, 125, 125),
        (Block::Air, _) => Color::NONE,
    };
    srgb.to_linear().to_f32_array()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_wind_counter_clockwise_around_normal() {
        for face in &FACES {
            let c = face.corners.map(Vec3::from);
            for [a, b, d] in [[0, 1, 2], [0, 2, 3]] {
                assert_eq!((c[b] - c[a]).cross(c[d] - c[a]), face.normal.as_vec3());
            }
        }
    }

    #[test]
    fn culls_hidden_faces() {
        let mut chunk = Chunk::default();
        chunk.set(IVec3::ZERO, Block::Stone);
        let mesh = chunk_mesh(&PaddedChunk::new(&chunk, |_| None));
        assert_eq!(mesh.count_vertices(), 6 * 4);

        chunk.set(IVec3::X, Block::Stone);
        let mesh = chunk_mesh(&PaddedChunk::new(&chunk, |_| None));
        assert_eq!(mesh.count_vertices(), 10 * 4);
    }

    #[test]
    fn neighbour_chunk_hides_border_face() {
        let mut chunk = Chunk::default();
        chunk.set(IVec3::new(CHUNK_SIZE - 1, 0, 0), Block::Stone);
        let mut east = Chunk::default();
        east.set(IVec3::ZERO, Block::Stone);

        let alone = chunk_mesh(&PaddedChunk::new(&chunk, |_| None));
        let joined = chunk_mesh(&PaddedChunk::new(&chunk, |o| (o == IVec3::X).then_some(&east)));
        assert_eq!(alone.count_vertices(), 6 * 4);
        assert_eq!(joined.count_vertices(), 5 * 4);
    }
}
