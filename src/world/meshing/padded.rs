use amity_protocol::world::{
    block::Block,
    chunk::Chunk,
    config::CHUNK_SIZE,
};
use bevy::prelude::*;

use super::mesher::FACES;

const PADDED_SIZE: i32 = CHUNK_SIZE + 2;

/// A chunk plus a one-block border copied from its six face neighbours, like
/// Minecraft's section copy. It is owned, so meshing can run on another thread
/// while the live world keeps changing, and every neighbour check is one array read.
pub struct PaddedChunk {
    blocks: Box<[Block]>,
}

impl PaddedChunk {
    /// `neighbour` returns the chunk at a chunk offset such as `IVec3::X`, if it is
    /// loaded. Missing neighbours count as air.
    /// Edges and corners of the border stay air: face culling never looks diagonally.
    pub fn new<'a>(center: &Chunk, neighbour: impl Fn(IVec3) -> Option<&'a Chunk>) -> Self {
        let mut blocks = vec![Block::AIR; (PADDED_SIZE * PADDED_SIZE * PADDED_SIZE) as usize];
        // interior: rows along x are contiguous in both layouts, so fill a row at a time
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let start = Self::index(IVec3::new(0, y, z));
                center.read_row(y, z, &mut blocks[start..start + CHUNK_SIZE as usize]);
            }
        }
        // border: one layer from each face neighbour, looked up once per face
        for face in &FACES {
            let Some(neighbour) = neighbour(face.normal) else {
                continue;
            };
            let edge = if face.normal.max_element() > 0 { CHUNK_SIZE } else { -1 };
            for i in 0..CHUNK_SIZE {
                for j in 0..CHUNK_SIZE {
                    let padded = if face.normal.x != 0 {
                        IVec3::new(edge, i, j)
                    } else if face.normal.y != 0 {
                        IVec3::new(i, edge, j)
                    } else {
                        IVec3::new(i, j, edge)
                    };
                    // -1 wraps to the neighbour's last layer, CHUNK_SIZE to its first
                    let local = padded.rem_euclid(IVec3::splat(CHUNK_SIZE));
                    blocks[Self::index(padded)] = neighbour.get(local);
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
