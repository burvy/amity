//! Builds a chunk's mesh: one textured square per visible block face

use amity_protocol::world::{
    block::Block,
    config::{BLOCK_SIZE, CHUNK_SIZE},
};
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

/// A chunk's two meshes. They're drawn with different materials: fluids are
/// see-through, and see-through surfaces must be drawn after (and sorted apart
/// from) everything solid
pub struct ChunkMeshes {
    /// every solid block's visible faces
    pub solid: Mesh,
    /// every fluid block's surfaces: only where the fluid meets air
    pub fluid: Mesh,
}

/// Face-culled meshes: a face is emitted only where it can be seen, including
/// across chunk borders.
///
/// - a solid block's face shows unless the block next to it is solid. So stone
///   under water still draws its faces, which you see through the water
/// - a fluid block's face shows only where it touches air. Water next to water
///   draws nothing, so a lake is one surface, not a grid of cubes, and water
///   against stone is hidden by the stone
pub fn chunk_mesh(chunk: &PaddedChunk) -> ChunkMeshes {
    let mut solid = MeshBuilder::default();
    let mut fluid = MeshBuilder::default();

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let local = IVec3::new(x, y, z);
                let block = chunk.get(local);
                for face in &FACES {
                    let neighbour = chunk.get(local + face.normal);
                    if block.is_solid() && !neighbour.is_solid() {
                        solid.face(local, face, face_tile(block, face.normal));
                    } else if block.is_fluid() && neighbour == Block::AIR {
                        fluid.face(local, face, face_tile(block, face.normal));
                    }
                }
            }
        }
    }

    ChunkMeshes {
        solid: solid.build(),
        fluid: fluid.build(),
    }
}

/// Collects faces into one mesh's vertex lists
#[derive(Default)]
struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    /// Adds one block face: 4 corners, 2 triangles, textured with atlas `tile`
    fn face(&mut self, local: IVec3, face: &Face, tile: u32) {
        let base = self.positions.len() as u32;
        let offset = local.as_vec3();
        for corner in face.corners {
            self.positions.push(((offset + Vec3::from(corner)) * BLOCK_SIZE).to_array());
        }
        self.normals.extend([face.normal.as_vec3().to_array(); 4]);
        self.uvs.extend(FACE_UVS.map(|uv| tile_uv(tile, uv)));
        // two triangles sharing the diagonal from corner 0 to corner 2
        self.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn build(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

#[cfg(test)]
mod tests {
    use amity_protocol::world::chunk::Chunk;

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
        chunk.set(IVec3::ZERO, Block::STONE);
        let mesh = chunk_mesh(&PaddedChunk::new(&chunk, |_| None)).solid;
        assert_eq!(mesh.count_vertices(), 6 * 4);

        chunk.set(IVec3::X, Block::STONE);
        let mesh = chunk_mesh(&PaddedChunk::new(&chunk, |_| None)).solid;
        assert_eq!(mesh.count_vertices(), 10 * 4);
    }

    #[test]
    fn every_neighbour_hides_its_shared_face() {
        for face in &FACES {
            // a block against this face of the chunk, and one just across it
            let inside = (face.normal + IVec3::ONE) * (CHUNK_SIZE - 1) / 2;
            let across = inside + face.normal - face.normal * CHUNK_SIZE;
            let mut chunk = Chunk::default();
            chunk.set(inside, Block::STONE);
            let mut other = Chunk::default();
            other.set(across, Block::STONE);

            let padded = PaddedChunk::new(&chunk, |o| (o == face.normal).then_some(&other));
            assert_eq!(chunk_mesh(&padded).solid.count_vertices(), 5 * 4, "{:?}", face.normal);
        }
    }

    #[test]
    fn neighbour_chunk_hides_border_face() {
        let mut chunk = Chunk::default();
        chunk.set(IVec3::new(CHUNK_SIZE - 1, 0, 0), Block::STONE);
        let mut east = Chunk::default();
        east.set(IVec3::ZERO, Block::STONE);

        let alone = chunk_mesh(&PaddedChunk::new(&chunk, |_| None)).solid;
        let joined = chunk_mesh(&PaddedChunk::new(&chunk, |o| (o == IVec3::X).then_some(&east))).solid;
        assert_eq!(alone.count_vertices(), 6 * 4);
        assert_eq!(joined.count_vertices(), 5 * 4);
    }

    #[test]
    fn solid_chunk_surrounded_by_solid_has_no_faces() {
        let stone = Chunk::Uniform(Block::STONE);
        let mesh = chunk_mesh(&PaddedChunk::new(&stone, |_| Some(&stone))).solid;
        assert_eq!(mesh.count_vertices(), 0);
    }

    #[test]
    fn water_shows_only_where_it_meets_air() {
        // two water blocks side by side, stone underneath one of them
        let mut chunk = Chunk::default();
        chunk.set(IVec3::new(1, 1, 1), Block::WATER);
        chunk.set(IVec3::new(2, 1, 1), Block::WATER);
        chunk.set(IVec3::new(1, 0, 1), Block::STONE);
        let meshes = chunk_mesh(&PaddedChunk::new(&chunk, |_| None));
        // 2 water blocks * 6 faces, minus the 2 they share, minus the 1 on stone
        assert_eq!(meshes.fluid.count_vertices(), 9 * 4);
        // the stone shows all 6 faces: water doesn't hide them
        assert_eq!(meshes.solid.count_vertices(), 6 * 4);
    }
}
