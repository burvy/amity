//! Builds a chunk's mesh: one textured square per visible block face

use std::collections::HashMap;

use amity_protocol::world::config::{BLOCK_SIZE, CHUNK_SIZE};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use super::{
    padded::PaddedChunk,
    textures::{face_tile, tile_uv},
};

/// One side of a unit block. Corners are counter-clockwise seen from outside
/// (bevy's front face), in the order bottom-left, bottom-right, top-right, top-left.
/// faces are not drawn from the inside and are drawn from the outside.
pub struct Face {
    pub normal: IVec3,
    corners: [[f32; 3]; 4],
}

pub const FACES: [Face; 6] = [
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

/// Matches `Face::corners`. Every face gets the whole 0..1 square, which
/// `tile_uv` then maps into the block's atlas tile.
const FACE_UVS: [[f32; 2]; 4] = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];

/// Face-culled mesh: a face is emitted only where the neighbouring block is air,
/// including across chunk borders.
pub fn chunk_mesh(chunk: &PaddedChunk) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    // each (block, face) pair's tile is looked up by name once per mesh
    let mut tiles = HashMap::new();

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
                        positions.push(((offset + Vec3::from(corner)) * BLOCK_SIZE).to_array());
                    }
                    normals.extend([face.normal.as_vec3().to_array(); 4]);
                    let tile = *tiles
                        .entry((block as u8, face.normal))
                        .or_insert_with(|| face_tile(block, face.normal));
                    uvs.extend(FACE_UVS.map(|uv| tile_uv(tile, uv)));
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
    .with_inserted_indices(Indices::U32(indices))
}

#[cfg(test)]
mod tests {
    use amity_protocol::world::{block::Block, chunk::Chunk};

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
    fn every_neighbour_hides_its_shared_face() {
        for face in &FACES {
            // a block against this face of the chunk, and one just across it
            let inside = (face.normal + IVec3::ONE) * (CHUNK_SIZE - 1) / 2;
            let across = inside + face.normal - face.normal * CHUNK_SIZE;
            let mut chunk = Chunk::default();
            chunk.set(inside, Block::Stone);
            let mut other = Chunk::default();
            other.set(across, Block::Stone);

            let padded = PaddedChunk::new(&chunk, |o| (o == face.normal).then_some(&other));
            assert_eq!(chunk_mesh(&padded).count_vertices(), 5 * 4, "{:?}", face.normal);
        }
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

    #[test]
    fn solid_chunk_surrounded_by_solid_has_no_faces() {
        let stone = Chunk::Uniform(Block::Stone);
        let mesh = chunk_mesh(&PaddedChunk::new(&stone, |_| Some(&stone)));
        assert_eq!(mesh.count_vertices(), 0);
    }
}
