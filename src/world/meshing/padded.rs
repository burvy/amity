use amity_protocol::world::{
    block::Block,
    chunk::{Chunk, ChunkSky},
    config::CHUNK_SIZE,
};
use bevy::prelude::*;

use super::light::sky_light;

pub(super) const PADDED_SIZE: i32 = CHUNK_SIZE + 2;
pub(super) const PADDED_VOLUME: usize = (PADDED_SIZE * PADDED_SIZE * PADDED_SIZE) as usize;

/// The 26 chunks around a chunk (faces, edges and corners), as offsets
pub fn neighbour_offsets() -> impl Iterator<Item = IVec3> {
    (0..27)
        .map(|i| IVec3::new(i % 3, i / 3 % 3, i / 9) - IVec3::ONE)
        .filter(|&offset| offset != IVec3::ZERO)
}

/// Where a chunk offset (each axis -1..=1) is kept in a 27-slot array
fn slot(offset: IVec3) -> usize {
    let i = offset + IVec3::ONE;
    (i.x + i.y * 3 + i.z * 9) as usize
}

/// Copies of a chunk and the 26 around it, with where each is open to the sky:
/// everything meshing reads, owned, so it can run on another thread while the
/// live world keeps changing. Copying is cheap: most chunks are a single block
/// type, and the rest are packed (see protocol `chunk/storage.rs`)
pub struct Neighbourhood {
    chunks: [Option<Chunk>; 27],
    skies: [Option<ChunkSky>; 27],
}

impl Neighbourhood {
    /// `around` returns the chunk (and its sky) at a chunk offset, IVec3::ZERO
    /// being the chunk itself, if it's loaded
    pub fn gather<'a>(around: impl Fn(IVec3) -> Option<(&'a Chunk, Option<&'a ChunkSky>)>) -> Self {
        let mut chunks = [const { None }; 27];
        let mut skies = [const { None }; 27];
        for offset in neighbour_offsets().chain([IVec3::ZERO]) {
            if let Some((chunk, sky)) = around(offset) {
                chunks[slot(offset)] = Some(chunk.clone());
                skies[slot(offset)] = sky.cloned();
            }
        }
        Self { chunks, skies }
    }

    /// Just blocks, open to the sky everywhere (for tests)
    #[cfg(test)]
    pub fn new<'a>(center: &Chunk, neighbour: impl Fn(IVec3) -> Option<&'a Chunk>) -> Self {
        Self::gather(|offset| {
            if offset == IVec3::ZERO {
                Some((center, None))
            } else {
                neighbour(offset).map(|chunk| (chunk, None))
            }
        })
    }

    /// Which chunk `p` (local to the middle chunk, -32..64 on each axis) is in,
    /// and where in it
    fn locate(p: IVec3) -> (usize, IVec3) {
        let size = IVec3::splat(CHUNK_SIZE);
        (slot(p.div_euclid(size)), p.rem_euclid(size))
    }

    /// The block at `p`. Missing chunks count as air
    pub fn block(&self, p: IVec3) -> Block {
        let (slot, local) = Self::locate(p);
        self.chunks[slot]
            .as_ref()
            .map_or(Block::AIR, |chunk| chunk.get(local))
    }

    /// Whether `p` has only air above it. Without sky data (a missing chunk,
    /// or tests), everything is open
    pub fn open(&self, p: IVec3) -> bool {
        let (slot, local) = Self::locate(p);
        self.skies[slot].as_ref().is_none_or(|sky| sky.is_open(local))
    }

    /// The middle chunk
    fn center(&self) -> Option<&Chunk> {
        self.chunks[slot(IVec3::ZERO)].as_ref()
    }
}

/// A chunk plus a one-block border from the chunks around it, like Minecraft's
/// section copy, and the sky light of each of those blocks. Every neighbour
/// check while meshing is one array read. The border's edges and corners matter
/// too: ambient occlusion and smooth light look diagonally
pub struct PaddedChunk {
    blocks: Box<[Block]>,
    light: Box<[u8]>,
}

impl PaddedChunk {
    pub fn new(area: &Neighbourhood) -> Self {
        let mut blocks = vec![Block::AIR; PADDED_VOLUME];
        // interior: rows along x are contiguous in both layouts, so fill a row at a time
        if let Some(center) = area.center() {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let start = Self::index(IVec3::new(0, y, z));
                    center.read_row(y, z, &mut blocks[start..start + CHUNK_SIZE as usize]);
                }
            }
        }
        // border: the one-block shell around it
        let range = -1..=CHUNK_SIZE;
        for y in range.clone() {
            for z in range.clone() {
                for x in range.clone() {
                    let p = IVec3::new(x, y, z);
                    if p.cmplt(IVec3::ZERO).any() || p.cmpge(IVec3::splat(CHUNK_SIZE)).any() {
                        blocks[Self::index(p)] = area.block(p);
                    }
                }
            }
        }
        Self {
            blocks: blocks.into_boxed_slice(),
            light: sky_light(area),
        }
    }

    /// same layout as `Chunk`, shifted by one so -1..=CHUNK_SIZE fits
    pub(super) fn index(p: IVec3) -> usize {
        let p = p + IVec3::ONE;
        (p.x + p.z * PADDED_SIZE + p.y * PADDED_SIZE * PADDED_SIZE) as usize
    }

    /// `p` is in the center chunk's local space, -1..=CHUNK_SIZE on each axis
    pub fn get(&self, p: IVec3) -> Block {
        self.blocks[Self::index(p)]
    }

    /// Sky light at `p` (as `get`): 0 (none, or solid) to `SKY_LIGHT_MAX`
    pub fn light(&self, p: IVec3) -> u8 {
        self.light[Self::index(p)]
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
        let area = Neighbourhood::new(&Chunk::default(), |offset| {
            (offset == IVec3::ONE).then_some(&corner)
        });
        let padded = PaddedChunk::new(&area);
        assert_eq!(padded.get(IVec3::splat(CHUNK_SIZE)), Block::STONE);
        assert_eq!(padded.get(IVec3::splat(-1)), Block::AIR);
    }
}
