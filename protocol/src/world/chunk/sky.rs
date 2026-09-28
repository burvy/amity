//! Where a chunk is open to the sky. Sky light (drawn by the client) starts at
//! every block that has nothing above it, all the way up, and spreads from
//! there into caves, under overhangs and down through water.
//!
//! The server knows this for every column of the world: its height, the first
//! block up from which there's only air (the top of the ground, or of water
//! standing on it). The client only has the chunks near it, so it couldn't tell
//! a cave from open ground if a mountain above it is out of range. So the server
//! sends each chunk's share of those heights with it (`ChunkData`), and the new
//! height of a column whenever an edit changes it (`BlockChanged`).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::world::config::CHUNK_SIZE;

/// Columns in a chunk
const COLUMNS: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;

/// For each of a chunk's 32 x 32 columns, the chunk-local height from which up
/// is open sky: 0 is the whole column, 32 none of it. Most chunks are one or
/// the other everywhere (open air above the ground, underground), stored as a
/// single number
#[derive(Component, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ChunkSky {
    Uniform(u8),
    /// per column, x + z * 32
    Columns(Vec<u8>),
}

impl ChunkSky {
    /// From each column's world height (x + z * 32), for the chunk at chunk
    /// height `chunk_y`
    pub fn from_heights(chunk_y: i32, heights: &[i32]) -> Self {
        let bottom = chunk_y * CHUNK_SIZE;
        let local: Vec<u8> = heights
            .iter()
            .map(|&height| (height - bottom).clamp(0, CHUNK_SIZE) as u8)
            .collect();
        match local.first() {
            Some(&first) if local.iter().all(|&h| h == first) => Self::Uniform(first),
            _ => Self::Columns(local),
        }
    }

    /// The local height from which column (x, z) is open sky
    pub fn open_from(&self, x: i32, z: i32) -> i32 {
        match self {
            Self::Uniform(height) => *height as i32,
            Self::Columns(heights) => heights[(x + z * CHUNK_SIZE) as usize] as i32,
        }
    }

    /// Whether the block at `local` has only air above it
    pub fn is_open(&self, local: IVec3) -> bool {
        local.y >= self.open_from(local.x, local.z)
    }

    /// Sets column (x, z)'s world height, for the chunk at chunk height
    /// `chunk_y`. Returns whether this chunk's share of it changed
    pub fn set_height(&mut self, chunk_y: i32, x: i32, z: i32, height: i32) -> bool {
        let local = (height - chunk_y * CHUNK_SIZE).clamp(0, CHUNK_SIZE) as u8;
        if self.open_from(x, z) == local as i32 {
            return false;
        }
        if let Self::Uniform(all) = *self {
            *self = Self::Columns(vec![all; COLUMNS]);
        }
        if let Self::Columns(heights) = self {
            heights[(x + z * CHUNK_SIZE) as usize] = local;
        }
        true
    }

    /// About how many bytes this takes to send
    pub fn byte_size(&self) -> usize {
        match self {
            Self::Uniform(_) => 1,
            Self::Columns(heights) => heights.len(),
        }
    }
}

/// An edit changed a column's height (see the module docs): it was `was`, and
/// is now `now`. Every chunk of the column between the two sees the sky
/// differently
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ColumnSky {
    pub was: i32,
    pub now: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_become_local_and_compact() {
        // ground at 10 everywhere: the chunk from 0 to 31 is open from 10 up
        let flat = vec![10; COLUMNS];
        assert_eq!(ChunkSky::from_heights(0, &flat), ChunkSky::Uniform(10));
        // the chunk above is all open, the one below all covered
        assert_eq!(ChunkSky::from_heights(1, &flat), ChunkSky::Uniform(0));
        assert_eq!(ChunkSky::from_heights(-1, &flat), ChunkSky::Uniform(32));
    }

    #[test]
    fn setting_a_height_splits_a_uniform_chunk() {
        let mut sky = ChunkSky::Uniform(10);
        assert!(sky.set_height(0, 3, 4, 20));
        assert_eq!(sky.open_from(3, 4), 20);
        assert_eq!(sky.open_from(0, 0), 10);
        assert!(!sky.is_open(IVec3::new(3, 15, 4)));
        assert!(sky.is_open(IVec3::new(0, 15, 0)));
        // the same again changes nothing
        assert!(!sky.set_height(0, 3, 4, 20));
    }
}
