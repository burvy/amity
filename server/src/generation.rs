use bevy::prelude::*;
use amity_protocol::world::chunk::{Block, Chunk, ChunkPos, CHUNK_SIZE};

/// Flat test terrain: the chunk layer at y = -1 is solid, so the ground's top
/// face sits at world y = 0. Everything else is air.
pub fn generate(pos: ChunkPos) -> Chunk {
    let mut chunk = Chunk::default();
    if pos.0.y != -1 {
        return chunk;
    }

    for y in 0..CHUNK_SIZE {
        let block = match y {
            31 => Block::Grass,
            28..=30 => Block::Dirt,
            _ => Block::Stone,
        };
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set(IVec3::new(x, y, z), block);
            }
        }
    }
    chunk
}
