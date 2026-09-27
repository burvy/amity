//! Turns settings and a chunk position into blocks: the part to edit to change
//! what the world looks like. Runs on a background thread, so it may only use
//! its arguments, never the ECS world.

use amity_protocol::world::{
    block::Block,
    chunk::{Chunk, ChunkPos},
    config::CHUNK_SIZE,
};
use bevy::prelude::*;

use super::settings::WorldGenSettings;

/// Flat terrain: grass on top, a few layers of dirt, then stone forever
pub fn generate(settings: &WorldGenSettings, pos: ChunkPos) -> Chunk {
    // world block heights of the chunk's lowest and highest layers
    let bottom = pos.0.y * CHUNK_SIZE;
    let top = bottom + CHUNK_SIZE - 1;
    // chunks entirely above the ground or below the dirt are a single block,
    // so they skip the per-block work and are stored as one byte
    if layer(settings, bottom) == layer(settings, top) {
        return Chunk::Uniform(layer(settings, bottom));
    }

    let mut chunk = Chunk::default();
    for y in 0..CHUNK_SIZE {
        let block = layer(settings, bottom + y);
        if block == Block::Air {
            continue;
        }
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set(IVec3::new(x, y, z), block);
            }
        }
    }
    chunk
}

/// The block at world block height `y`. Every layer below the grass is at least
/// as deep as the one above, so equal blocks at a chunk's top and bottom mean the
/// whole chunk is that block
fn layer(settings: &WorldGenSettings, y: i32) -> Block {
    let depth = settings.ground_level - 1 - y; // blocks below the grass
    match depth {
        ..0 => Block::Air,
        0 => Block::Grass,
        d if d <= settings.dirt_depth => Block::Dirt,
        _ => Block::Stone,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_top_sits_at_ground_level() {
        let settings = WorldGenSettings::default();
        let surface = generate(&settings, ChunkPos(IVec3::new(5, -1, -3)));
        let top = CHUNK_SIZE - 1;
        assert_eq!(surface.get(IVec3::new(0, top, 0)), Block::Grass);
        assert_eq!(surface.get(IVec3::new(9, top - 1, 4)), Block::Dirt);
        assert_eq!(surface.get(IVec3::new(9, top - 4, 4)), Block::Stone);

        let sky = generate(&settings, ChunkPos(IVec3::ZERO));
        let deep = generate(&settings, ChunkPos(IVec3::new(0, -2, 0)));
        assert_eq!(sky, Chunk::Uniform(Block::Air));
        assert_eq!(deep, Chunk::Uniform(Block::Stone));
    }
}
