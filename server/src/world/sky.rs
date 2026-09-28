//! Every column's height: the first block up from which there's only air (see
//! protocol `chunk/sky.rs`, which is what the client uses it for).
//!
//! World generation knows it for a column before any of its chunks exist (the
//! top of the ground or of the water on it), so it's stored as soon as the
//! column's first chunk is generated. Edits then move it: placing a block above
//! it raises it, breaking the top block lowers it to the next block down.

use amity_protocol::world::{
    block::Block,
    blocks::Blocks,
    chunk::{ChunkPos, ChunkSky, ColumnSky},
    config::CHUNK_SIZE,
};
use bevy::{platform::collections::HashMap, prelude::*};

/// Heights per chunk column (x, z in chunks), each 32 x 32 (x + z * 32)
#[derive(Resource, Default)]
pub struct SkyHeights(HashMap<IVec2, Box<[i32]>>);

/// A chunk's column, and a block's place in it
fn column_of(pos: IVec3) -> (IVec2, usize) {
    let column = IVec2::new(pos.x.div_euclid(CHUNK_SIZE), pos.z.div_euclid(CHUNK_SIZE));
    let local = pos.rem_euclid(IVec3::splat(CHUNK_SIZE));
    (column, (local.x + local.z * CHUNK_SIZE) as usize)
}

impl SkyHeights {
    /// Stores a column's heights from world generation, unless it has them
    /// already (every chunk of a column generates the same heights, and edits
    /// may have changed them since)
    pub fn generated(&mut self, chunk: ChunkPos, heights: Vec<i32>) {
        let column = IVec2::new(chunk.0.x, chunk.0.z);
        self.0
            .entry(column)
            .or_insert_with(|| heights.into_boxed_slice());
    }

    /// What the chunk at `chunk` sees of the sky. A column not generated yet
    /// counts as open (it can't happen: a chunk is generated before it's sent)
    pub fn chunk_sky(&self, chunk: ChunkPos) -> ChunkSky {
        match self.0.get(&IVec2::new(chunk.0.x, chunk.0.z)) {
            Some(heights) => ChunkSky::from_heights(chunk.0.y, heights),
            None => ChunkSky::Uniform(0),
        }
    }

    /// After the block at `pos` became `block`: moves its column's height if
    /// it has to, and returns the change
    pub fn edited(&mut self, pos: IVec3, block: Block, blocks: &Blocks) -> Option<ColumnSky> {
        let (column, i) = column_of(pos);
        let heights = self.0.get_mut(&column)?;
        let was = heights[i];
        let now = if block != Block::AIR {
            // placed: the column is covered at least up to it
            was.max(pos.y + 1)
        } else if pos.y + 1 == was {
            // the top block went: down to the next block that isn't air. Where
            // the world isn't generated (far below), stop and count it as covered
            let mut y = pos.y - 1;
            while blocks.get(IVec3::new(pos.x, y, pos.z)) == Some(Block::AIR) {
                y -= 1;
            }
            y + 1
        } else {
            was
        };
        if now == was {
            return None;
        }
        heights[i] = now;
        Some(ColumnSky { was, now })
    }
}
