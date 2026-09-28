use amity_protocol::world::{block::Block, chunk::Chunk, config::CHUNK_SIZE};
use bevy::prelude::*;

const PADDED_SIZE: i32 = CHUNK_SIZE + 2;

/// The 26 chunks around a chunk (faces, edges and corners), as offsets
pub fn neighbour_offsets() -> impl Iterator<Item = IVec3> {
    (0..27)
        .map(|i| IVec3::new(i % 3, i / 3 % 3, i / 9) - IVec3::ONE)
        .filter(|&offset| offset != IVec3::ZERO)
}

/// A chunk plus a one-block border copied from the 26 chunks around it, like
/// Minecraft's section copy. It is owned, so meshing can run on another thread
/// while the live world keeps changing, and every neighbour check is one array
/// read. The border's edges and corners matter too: ambient occlusion looks
/// diagonally
pub struct PaddedChunk {
    blocks: Box<[Block]>,
}

impl PaddedChunk {
    /// `neighbour` returns the chunk at a chunk offset such as `IVec3::X`, if it
    /// is loaded. Missing neighbours count as air.
    pub fn new<'a>(center: &Chunk, neighbour: impl Fn(IVec3) -> Option<&'a Chunk>) -> Self {
        let mut blocks = vec![Block::AIR; (PADDED_SIZE * PADDED_SIZE * PADDED_SIZE) as usize];
        // interior: rows along x are contiguous in both layouts, so fill a row at a time
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let start = Self::index(IVec3::new(0, y, z));
                center.read_row(y, z, &mut blocks[start..start + CHUNK_SIZE as usize]);
            }
        }
        // border: each neighbour looked up once, by offset + 1 (so 0..=2)
        let mut around = [None; 27];
        for offset in neighbour_offsets() {
            let i = offset + IVec3::ONE;
            around[(i.x + i.y * 3 + i.z * 9) as usize] = neighbour(offset);
        }
        let range = -1..=CHUNK_SIZE;
        for y in range.clone() {
            for z in range.clone() {
                for x in range.clone() {
                    let padded = IVec3::new(x, y, z);
                    // -1 is the neighbour below's last layer, CHUNK_SIZE the one above's first
                    let offset = padded.div_euclid(IVec3::splat(CHUNK_SIZE));
                    if offset == IVec3::ZERO {
                        continue;
                    }
                    let i = offset + IVec3::ONE;
                    if let Some(chunk) = around[(i.x + i.y * 3 + i.z * 9) as usize] {
                        let local = padded.rem_euclid(IVec3::splat(CHUNK_SIZE));
                        blocks[Self::index(padded)] = chunk.get(local);
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_26_neighbours() {
        assert_eq!(neighbour_offsets().count(), 26);
    }

    #[test]
    fn corners_come_from_the_diagonal_chunk() {
        let mut corner = Chunk::default();
        corner.set(IVec3::ZERO, Block::STONE);
        let padded = PaddedChunk::new(&Chunk::default(), |offset| {
            (offset == IVec3::ONE).then_some(&corner)
        });
        assert_eq!(padded.get(IVec3::splat(CHUNK_SIZE)), Block::STONE);
        assert_eq!(padded.get(IVec3::splat(-1)), Block::AIR);
    }
}
