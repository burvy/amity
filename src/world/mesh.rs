use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use shooter_protocol::world::chunk::{Block, Chunk, CHUNK_SIZE};

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
        perceptual_roughness: 1.0,
        ..default()
    })));
}

/// (Re)meshes chunks when they arrive or change
pub fn mesh_chunks(
    mut cmds: Commands,
    chunks: Query<(Entity, &Chunk), Changed<Chunk>>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<ChunkMaterial>,
) {
    for (entity, chunk) in &chunks {
        cmds.entity(entity).insert((
            Mesh3d(meshes.add(chunk_mesh(chunk))),
            MeshMaterial3d(material.0.clone()),
        ));
    }
}

/// Face-culled mesh: a face is emitted only where the neighbouring block is air.
/// Blocks outside this chunk count as air, so faces between two solid chunks
/// are still emitted (hidden, but drawn).
pub fn chunk_mesh(chunk: &Chunk) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for local in chunk.solid() {
        let block = chunk.get(local);
        for face in &FACES {
            let neighbour = local + face.normal;
            if in_chunk(neighbour) && chunk.get(neighbour).is_solid() {
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

fn in_chunk(local: IVec3) -> bool {
    local.cmpge(IVec3::ZERO).all() && local.cmplt(IVec3::splat(CHUNK_SIZE)).all()
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
            let [a, b, c, _] = face.corners.map(Vec3::from);
            assert_eq!((b - a).cross(c - a), face.normal.as_vec3());
        }
    }

    #[test]
    fn culls_hidden_faces() {
        let mut chunk = Chunk::default();
        chunk.set(IVec3::ZERO, Block::Stone);
        assert_eq!(chunk_mesh(&chunk).count_vertices(), 6 * 4);

        chunk.set(IVec3::X, Block::Stone);
        assert_eq!(chunk_mesh(&chunk).count_vertices(), 10 * 4);
    }
}
