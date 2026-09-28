//! Where chunks are, and finding a chunk's entity by position

use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

use crate::world::{
    block::{block_at, block_corner},
    config::CHUNK_SIZE,
};

/// world block = chunk * CHUNK_SIZE + local
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ChunkPos(pub IVec3);

impl ChunkPos {
    /// world position of this chunk's (0, 0, 0) block corner
    pub fn origin(self) -> Vec3 {
        block_corner(self.0 * CHUNK_SIZE)
    }

    /// splits a world block coordinate into its chunk and the local position inside it.
    /// euclidean division so -1 lands in chunk -1 at local 31, not chunk 0 at local -1
    pub fn of_block(world: IVec3) -> (ChunkPos, IVec3) {
        let size = IVec3::splat(CHUNK_SIZE);
        (ChunkPos(world.div_euclid(size)), world.rem_euclid(size))
    }

    /// the chunk containing a world point in metres
    pub fn containing(point: Vec3) -> ChunkPos {
        Self::of_block(block_at(point)).0
    }
}

/// Finds a chunk's entity by its position. Kept up to date by observers, so it
/// covers every chunk entity on both client and server.
#[derive(Resource, Default)]
pub struct ChunkIndex(HashMap<IVec3, Entity>);

impl ChunkIndex {
    pub fn get(&self, pos: ChunkPos) -> Option<Entity> {
        self.0.get(&pos.0).copied()
    }

    pub fn contains(&self, pos: ChunkPos) -> bool {
        self.0.contains_key(&pos.0)
    }

    /// How many chunks are loaded
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub fn index_chunk(add: On<Add, ChunkPos>, chunks: Query<&ChunkPos>, mut index: ResMut<ChunkIndex>) {
    if let Ok(pos) = chunks.get(add.entity) {
        index.0.insert(pos.0, add.entity);
    }
}

pub fn unindex_chunk(
    remove: On<Remove, ChunkPos>,
    chunks: Query<&ChunkPos>,
    mut index: ResMut<ChunkIndex>,
) {
    if let Ok(pos) = chunks.get(remove.entity) {
        index.0.remove(&pos.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_blocks_land_in_negative_chunks() {
        assert_eq!(
            ChunkPos::of_block(IVec3::new(-1, 0, 31)),
            (ChunkPos(IVec3::new(-1, 0, 0)), IVec3::new(31, 0, 31))
        );
        assert_eq!(
            ChunkPos::of_block(IVec3::splat(-CHUNK_SIZE)),
            (ChunkPos(IVec3::NEG_ONE), IVec3::ZERO)
        );
    }
}
