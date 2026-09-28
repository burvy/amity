//! Builds a chunk's meshes.
//!
//! 1. Visibility: a face is drawn only where it can be seen (see `chunk_mesh`).
//! 2. Ambient occlusion: each face corner is darkened by the blocks around it,
//!    so creases, corners and the foot of walls get soft shading. It's worked
//!    out here, once per mesh, and stored per vertex: it costs nothing to draw.
//!    Sky light (`light.rs`) is stored per corner the same way, averaged over
//!    the blocks around the corner so it fades smoothly across faces.
//! 3. Greedy meshing: neighbouring faces that look the same (same texture, same
//!    shading) are merged into one large rectangle. A flat field that was
//!    thousands of squares becomes a handful, which every render pass (the
//!    view, and each shadow cascade) draws faster. The texture still repeats
//!    once per block: the block shader (`block.wgsl`) tiles it across the
//!    rectangle.

use amity_protocol::world::config::{BLOCK_SIZE, CHUNK_SIZE};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use super::{
    config::{AO_BRIGHTNESS, SKY_LIGHT_MAX, SKY_LIGHT_POWER},
    padded::PaddedChunk,
    textures::{face_tile, tile_corner},
};

/// One side of a block. `u` and `v` run across the face: a rectangle's corners
/// go (0,0), (1,0), (1,1), (0,1) in (u, v), and since `u` x `v` = `normal`,
/// that's counter-clockwise seen from outside (bevy's front face)
pub struct Face {
    pub normal: IVec3,
    u: IVec3,
    v: IVec3,
}

pub const FACES: [Face; 6] = [
    Face { normal: IVec3::X, u: IVec3::Y, v: IVec3::Z },
    Face { normal: IVec3::NEG_X, u: IVec3::Z, v: IVec3::Y },
    Face { normal: IVec3::Y, u: IVec3::Z, v: IVec3::X },
    Face { normal: IVec3::NEG_Y, u: IVec3::X, v: IVec3::Z },
    Face { normal: IVec3::Z, u: IVec3::X, v: IVec3::Y },
    Face { normal: IVec3::NEG_Z, u: IVec3::Y, v: IVec3::X },
];

/// The corners of a face, in (u, v), in winding order
const CORNERS: [IVec2; 4] = [
    IVec2::new(0, 0),
    IVec2::new(1, 0),
    IVec2::new(1, 1),
    IVec2::new(0, 1),
];

/// A chunk's three meshes, each drawn with its own material: fluids are
/// blended with what's behind them, which must be drawn after (and sorted
/// apart from) everything solid; see-through blocks like leaves cut holes
/// where their texture is transparent, which is cheap but still costs more
/// than solid blocks, so they're kept apart from those
pub struct ChunkMeshes {
    /// every solid block's visible faces
    pub solid: Mesh,
    /// every see-through block's visible faces
    pub cutout: Mesh,
    /// every fluid block's surfaces: only where the fluid meets air
    pub fluid: Mesh,
}

/// Which of the chunk's meshes a face goes in
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Surface {
    Solid,
    Cutout,
    Fluid,
}

/// What a visible face looks like. Faces that match can be merged
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Look {
    surface: Surface,
    tile: u32,
    /// ambient occlusion per corner, in `CORNERS` order: 0 darkest, 3 open
    ao: [u8; 4],
    /// sky light per corner, in `CORNERS` order: the average level of the
    /// blocks around the corner, times 4 (so 0 to 4 * SKY_LIGHT_MAX)
    light: [u8; 4],
}

impl Look {
    /// Whether the shading changes along u (then a merge along u would stretch
    /// it wrongly), or along v
    fn varies_along_u(&self) -> bool {
        let differs = |c: [u8; 4]| c[0] != c[1] || c[3] != c[2];
        differs(self.ao) || differs(self.light)
    }
    fn varies_along_v(&self) -> bool {
        let differs = |c: [u8; 4]| c[0] != c[3] || c[1] != c[2];
        differs(self.ao) || differs(self.light)
    }
}

/// Visible, merged faces for both meshes.
///
/// Which faces are visible:
/// - a solid block's face shows unless the block next to it is opaque. So stone
///   under water or behind leaves still draws its faces, which you see through
///   the water or the leaves' gaps. Leaves next to leaves draw their faces too,
///   so a canopy looks full, not hollow
/// - a fluid block's face shows only where it touches air (or leaves). Water
///   next to water draws nothing, so a lake is one surface, not a grid of
///   cubes, and water against stone is hidden by the stone
///
/// For each face direction, the chunk is cut into slices one block thick. Each
/// slice's visible faces go in a 2D grid, which is then covered with as few
/// rectangles as it can: from each uncovered face, grow along u while the faces
/// match, then along v while whole rows match.
pub fn chunk_mesh(chunk: &PaddedChunk) -> ChunkMeshes {
    const N: i32 = CHUNK_SIZE;
    let mut solid = MeshBuilder::default();
    let mut cutout = MeshBuilder::default();
    let mut fluid = MeshBuilder::default();
    let mut grid: Vec<Option<Look>> = vec![None; (N * N) as usize];
    let cell = |a: i32, b: i32| (a + b * N) as usize;

    for face in &FACES {
        // the axis the slices are cut across (as a positive direction)
        let across = face.normal.abs();
        for slice in 0..N {
            // 1. this slice's visible faces
            for b in 0..N {
                for a in 0..N {
                    let block_pos = across * slice + face.u * a + face.v * b;
                    grid[cell(a, b)] = look(chunk, face, block_pos);
                }
            }

            // 2. covered with rectangles
            for b in 0..N {
                for a in 0..N {
                    let Some(look) = grid[cell(a, b)] else {
                        continue;
                    };
                    let mut width = 1;
                    if !look.varies_along_u() {
                        while a + width < N && grid[cell(a + width, b)] == Some(look) {
                            width += 1;
                        }
                    }
                    let mut height = 1;
                    if !look.varies_along_v() {
                        while b + height < N
                            && (0..width).all(|k| grid[cell(a + k, b + height)] == Some(look))
                        {
                            height += 1;
                        }
                    }
                    for dv in 0..height {
                        for du in 0..width {
                            grid[cell(a + du, b + dv)] = None;
                        }
                    }

                    let block_pos = across * slice + face.u * a + face.v * b;
                    let builder = match look.surface {
                        Surface::Solid => &mut solid,
                        Surface::Cutout => &mut cutout,
                        Surface::Fluid => &mut fluid,
                    };
                    builder.rectangle(face, block_pos, IVec2::new(width, height), look);
                }
            }
        }
    }

    ChunkMeshes {
        solid: solid.build(),
        cutout: cutout.build(),
        fluid: fluid.build(),
    }
}

/// How the face of the block at `pos` facing `face.normal` looks, if visible
fn look(chunk: &PaddedChunk, face: &Face, pos: IVec3) -> Option<Look> {
    let block = chunk.get(pos);
    let front = pos + face.normal;
    let neighbour = chunk.get(front);
    let surface = if block.is_solid() && !neighbour.is_opaque() {
        if block.is_see_through() {
            Surface::Cutout
        } else {
            Surface::Solid
        }
    } else if block.is_fluid() && !neighbour.is_fluid() && !neighbour.is_opaque() {
        Surface::Fluid
    } else {
        return None;
    };
    // what shades a corner: anything solid, leaves included
    let solid = |p: IVec3| chunk.get(p).is_solid();
    // what stops sky light: only blocks with no gaps
    let opaque = |p: IVec3| chunk.get(p).is_opaque();
    // towards each corner, along u and along v
    let towards = CORNERS.map(|corner| (face.u * (corner.x * 2 - 1), face.v * (corner.y * 2 - 1)));
    // a fluid's surface isn't shaded: it would show as dark patches on water
    let ao = if surface == Surface::Fluid {
        [3; 4]
    } else {
        towards.map(|(du, dv)| vertex_ao(solid(front + du), solid(front + dv), solid(front + du + dv)))
    };
    let light = towards.map(|(du, dv)| {
        // the blocks in front of the face around this corner, leaving out
        // opaque ones (they're dark inside), and the diagonal if both sides are
        // opaque (it's walled off from the corner)
        let (side_1, side_2) = (opaque(front + du), opaque(front + dv));
        let around = [
            Some(front),
            (!side_1).then_some(front + du),
            (!side_2).then_some(front + dv),
            (!(side_1 && side_2) && !opaque(front + du + dv)).then_some(front + du + dv),
        ];
        let (sum, count) = around
            .into_iter()
            .flatten()
            .fold((0u32, 0u32), |(sum, count), p| (sum + chunk.light(p) as u32, count + 1));
        // the average, times 4 and rounded (`front` is never opaque: count >= 1)
        ((sum * 4 + count / 2) / count) as u8
    });
    Some(Look {
        surface,
        tile: face_tile(block, face.normal),
        ao,
        light,
    })
}

/// A corner's ambient occlusion from the blocks in front of the face around
/// it: the two beside the corner and the one diagonal to it. 3 is open; 0 is
/// fully tucked in. Both sides solid is darkest whatever the diagonal is (the
/// corner is enclosed)
fn vertex_ao(side_1: bool, side_2: bool, diagonal: bool) -> u8 {
    if side_1 && side_2 {
        0
    } else {
        3 - (side_1 as u8 + side_2 as u8 + diagonal as u8)
    }
}

/// Collects rectangles into one mesh's vertex lists
#[derive(Default)]
struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    /// position across the face, in blocks: the shader repeats the texture
    /// once per whole number
    uvs: Vec<[f32; 2]>,
    /// the texture's tile in the atlas (its top-left corner)
    tiles: Vec<[f32; 2]>,
    /// rgb: ambient occlusion, as a brightness the texture is multiplied by.
    /// a: how much of the sky's light reaches here (0 to 1), see `block.wgsl`
    colours: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    /// Adds a rectangle of `size` faces (along u, v) whose first face belongs
    /// to the block at `block_pos`: 4 corners, 2 triangles
    fn rectangle(&mut self, face: &Face, block_pos: IVec3, size: IVec2, look: Look) {
        let base = self.positions.len() as u32;
        // along a positive normal, the face is on the block's far side
        let origin = block_pos + face.normal.max(IVec3::ZERO);
        let tile = tile_corner(look.tile);
        for ((corner, ao), light) in CORNERS.iter().zip(look.ao).zip(look.light) {
            let pos = (origin + face.u * corner.x * size.x + face.v * corner.y * size.y).as_vec3();
            self.positions.push((pos * BLOCK_SIZE).to_array());
            self.uvs.push(texture_uv(face.normal, pos));
            let brightness = AO_BRIGHTNESS[ao as usize];
            let sky = (light as f32 / (4 * SKY_LIGHT_MAX) as f32).powf(SKY_LIGHT_POWER);
            self.colours.push([brightness, brightness, brightness, sky]);
        }
        self.normals.extend([face.normal.as_vec3().to_array(); 4]);
        self.tiles.extend([tile; 4]);

        // The rectangle's two triangles share a diagonal. Shading is blended
        // across each triangle, so a dark corner on the diagonal would smear
        // along it: use the diagonal whose ends are brighter
        let ao = look.ao;
        let indices = if ao[0] + ao[2] >= ao[1] + ao[3] {
            [0, 1, 2, 0, 2, 3]
        } else {
            [1, 2, 3, 1, 3, 0]
        };
        self.indices.extend(indices.map(|i| base + i));
    }

    fn build(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, self.tiles)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colours)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Where on the texture a point of a face is, in blocks: the texture repeats
/// once per block. Taken from the point's position, so a texture continues
/// seamlessly across merged faces. Sides have the texture's top at the top (v
/// grows downwards in images), and nothing is mirrored seen from outside
fn texture_uv(normal: IVec3, p: Vec3) -> [f32; 2] {
    match normal {
        IVec3::Y => [p.x, p.z],
        IVec3::NEG_Y => [p.x, -p.z],
        IVec3::X => [-p.z, -p.y],
        IVec3::NEG_X => [p.z, -p.y],
        IVec3::Z => [p.x, -p.y],
        _ => [-p.x, -p.y],
    }
}

#[cfg(test)]
mod tests {
    use amity_protocol::world::{block::Block, chunk::Chunk};

    use super::*;
    use crate::world::meshing::padded::Neighbourhood;

    fn padded<'a>(chunk: &Chunk, neighbour: impl Fn(IVec3) -> Option<&'a Chunk>) -> PaddedChunk {
        PaddedChunk::new(&Neighbourhood::new(chunk, neighbour))
    }

    /// (solid, fluid) rectangles in a chunk alone (no neighbours)
    fn rectangles(chunk: &Chunk) -> (usize, usize) {
        let meshes = chunk_mesh(&padded(chunk, |_| None));
        (
            meshes.solid.count_vertices() / 4,
            meshes.fluid.count_vertices() / 4,
        )
    }

    #[test]
    fn corners_wind_counter_clockwise_around_normal() {
        for face in &FACES {
            assert_eq!(face.u.cross(face.v), face.normal);
        }
    }

    #[test]
    fn a_lone_block_has_six_faces() {
        let mut chunk = Chunk::default();
        chunk.set(IVec3::ZERO, Block::STONE);
        assert_eq!(rectangles(&chunk), (6, 0));
    }

    #[test]
    fn flat_faces_merge() {
        // two blocks side by side are still one box: 6 rectangles
        let mut chunk = Chunk::default();
        chunk.set(IVec3::ZERO, Block::STONE);
        chunk.set(IVec3::X, Block::STONE);
        assert_eq!(rectangles(&chunk), (6, 0));

        // a whole flat floor too
        let mut floor = Chunk::default();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                floor.set(IVec3::new(x, 0, z), Block::STONE);
            }
        }
        assert_eq!(rectangles(&floor), (6, 0));
    }

    #[test]
    fn different_textures_dont_merge() {
        let mut chunk = Chunk::default();
        chunk.set(IVec3::ZERO, Block::STONE);
        chunk.set(IVec3::X, Block::SAND);
        // the 4 long sides each split in two, plus the 2 ends
        assert_eq!(rectangles(&chunk), (10, 0));
    }

    /// see-through rectangles in a chunk alone
    fn cutout_rectangles(chunk: &Chunk) -> usize {
        chunk_mesh(&padded(chunk, |_| None)).cutout.count_vertices() / 4
    }

    #[test]
    fn leaves_show_what_is_behind_them() {
        // stone with leaves on top: the stone's top face still draws (it shows
        // through the gaps), and the leaves go in their own mesh, all but
        // their bottom, which the stone hides
        let mut chunk = Chunk::default();
        chunk.set(IVec3::ZERO, Block::STONE);
        chunk.set(IVec3::Y, Block::OAK_LEAVES);
        assert_eq!(rectangles(&chunk).0, 6);
        assert_eq!(cutout_rectangles(&chunk), 5);
    }

    #[test]
    fn leaves_next_to_leaves_draw_between_them() {
        // two leaf blocks: the faces between them draw too (one each way), so
        // a canopy isn't hollow when seen through its gaps. Plus the outside:
        // 4 long sides (merged) and 2 ends
        let mut chunk = Chunk::default();
        chunk.set(IVec3::ZERO, Block::OAK_LEAVES);
        chunk.set(IVec3::X, Block::OAK_LEAVES);
        assert_eq!(cutout_rectangles(&chunk), 8);
    }

    #[test]
    fn corners_darken_by_their_neighbours() {
        assert_eq!(vertex_ao(false, false, false), 3);
        assert_eq!(vertex_ao(false, false, true), 2);
        assert_eq!(vertex_ao(true, false, true), 1);
        assert_eq!(vertex_ao(true, true, false), 0);
    }

    #[test]
    fn a_block_on_a_floor_shades_the_floor_around_it() {
        let mut chunk = Chunk::default();
        for x in 0..5 {
            for z in 0..5 {
                chunk.set(IVec3::new(x, 0, z), Block::STONE);
            }
        }
        chunk.set(IVec3::new(2, 1, 2), Block::STONE);
        let padded = padded(&chunk, |_| None);
        let top = &FACES[2];
        // the floor face right beside the block: its two corners against it are dark
        let beside = look(&padded, top, IVec3::new(3, 0, 2)).unwrap();
        assert_eq!(beside.ao.iter().filter(|&&ao| ao < 3).count(), 2);
        // far from it: untouched
        let far = look(&padded, top, IVec3::new(0, 0, 0)).unwrap();
        assert_eq!(far.ao, [3; 4]);
        // under it: hidden
        assert!(look(&padded, top, IVec3::new(2, 0, 2)).is_none());
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

            let padded = padded(&chunk, |o| (o == face.normal).then_some(&other));
            assert_eq!(chunk_mesh(&padded).solid.count_vertices(), 5 * 4, "{:?}", face.normal);
        }
    }

    #[test]
    fn solid_chunk_surrounded_by_solid_has_no_faces() {
        let stone = Chunk::Uniform(Block::STONE);
        let mesh = chunk_mesh(&padded(&stone, |_| Some(&stone))).solid;
        assert_eq!(mesh.count_vertices(), 0);
    }

    #[test]
    fn water_shows_only_where_it_meets_air() {
        // two water blocks side by side, stone underneath one of them
        let mut chunk = Chunk::default();
        chunk.set(IVec3::new(1, 1, 1), Block::WATER);
        chunk.set(IVec3::new(2, 1, 1), Block::WATER);
        chunk.set(IVec3::new(1, 0, 1), Block::STONE);
        // water: one top, one bottom (the half over air), 2 long sides, 2 ends.
        // The stone shows all 6 faces: water doesn't hide them
        assert_eq!(rectangles(&chunk), (6, 6));
    }
}
