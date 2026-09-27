//! Turns settings and a chunk position into blocks: the code to edit to change
//! how the world is shaped. `settings.rs` holds the numbers.
//!
//! # How the ground is made
//!
//! 1. **Height per column.** Every (x, z) column of blocks gets one number, its
//!    ground height: `base_height` plus the sum of every noise layer at that spot.
//!    Noise is deterministic: the same seed and position always give the same
//!    value, so chunks can be generated in any order, on any thread, and still
//!    line up at their borders.
//! 2. **Blocks from height.** Going down a column from the ground: one grass
//!    block, `dirt_depth` dirt blocks, then stone forever. Above is air.
//!
//! Only height per column exists for now, so there are no overhangs or caves;
//! those need 3D noise, which comes later.
//!
//! # Fractal noise (fBm), per layer
//!
//! Perlin noise is a smooth random surface between -1 and 1. fBm adds several
//! copies ("octaves"), each smaller and weaker than the last:
//! The `noise` crate does the octave sum (`Fbm<Perlin>`); a layer then scales its
//! -1..1 output by `height` metres.
//! Essentially, a bunch of waves that get weaker, and all summed up to make cool terrain.

use std::sync::Arc;

use amity_protocol::world::{
    block::Block,
    chunk::{Chunk, ChunkPos},
    config::{BLOCK_SIZE, CHUNK_SIZE},
};
use bevy::prelude::*;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use super::settings::{NoiseLayer, WorldGenSettings};

/// The terrain generator, built once from the settings. Cloning it is cheap (it
/// is shared), so every background generation task gets its own handle.
///
/// Why an `Arc`: generation runs on background threads, and each task needs the
/// generator for as long as it runs. `Arc` lets every task hold the same one
/// without copying the noise tables; cloning `Terrain` only bumps a counter.
#[derive(Resource, Clone)]
pub struct Terrain(Arc<TerrainInner>);

/// What `Terrain` shares between tasks: the settings, with the noise built
struct TerrainInner {
    /// metres, see `WorldGenSettings::base_height`
    base_height: f64,
    /// blocks, see `WorldGenSettings::dirt_depth`
    dirt_depth: i32,
    layers: Vec<Layer>,
}

/// A `NoiseLayer` with its noise built and ready to sample
struct Layer {
    /// fBm: `octaves` Perlin noises summed, each smaller and weaker than the
    /// last. Returns roughly -1..1 for any point
    noise: Fbm<Perlin>,
    /// metres: what the -1..1 is multiplied by
    height: f64,
}

impl Layer {
    fn new(settings: &NoiseLayer, seed: u32) -> Self {
        Self {
            // `new(seed)` makes the random tables the noise is built from. The
            // `set_*` calls are the knobs from `NoiseLayer`
            noise: Fbm::<Perlin>::new(seed)
                .set_octaves(settings.octaves)
                // Perlin noise has about one bump per 1.0 of input. Multiplying
                // positions by 1 / scale means the input only moves 1.0 every
                // `scale` metres, so bumps come out `scale` metres wide
                .set_frequency(1.0 / settings.scale)
                .set_persistence(settings.persistence)
                .set_lacunarity(settings.lacunarity),
            height: settings.height,
        }
    }

    /// Metres this layer moves the ground at (x, z) metres.
    /// e.g. noise 0.4 with height 30 raises the ground 12 m; -0.4 lowers it 12 m
    fn sample(&self, x: f64, z: f64) -> f64 {
        self.noise.get([x, z]) * self.height
    }
}

impl Terrain {
    pub fn new(settings: &WorldGenSettings) -> Self {
        let layers = settings
            .layers
            .iter()
            .enumerate()
            // each layer gets its own seed (seed, seed + 1, ...): with the same
            // seed, two layers would have their bumps in the same places
            .map(|(i, layer)| Layer::new(layer, settings.seed.wrapping_add(i as u32)))
            .collect();
        Self(Arc::new(TerrainInner {
            base_height: settings.base_height,
            dirt_depth: settings.dirt_depth,
            layers,
        }))
    }

    /// Ground height of the block column at block `x`, `z`, in blocks: the y of
    /// the first air block. Sampled at the column's centre
    pub fn height_at(&self, x: i32, z: i32) -> i32 {
        // Noise works in metres, so settings mean the same thing whatever the
        // block size. Block 3 spans 1.5..2.0 m; its centre is (3 + 0.5) * 0.5 = 1.75 m
        let x = (x as f64 + 0.5) * BLOCK_SIZE as f64;
        let z = (z as f64 + 0.5) * BLOCK_SIZE as f64;

        // every layer's push up or down, added onto the base height.
        // e.g. base 0 + hills 12.3 + detail -0.6 = 11.7 m
        let metres: f64 = self.0.base_height
            + self
                .0
                .layers
                .iter()
                .map(|layer| layer.sample(x, z))
                .sum::<f64>();

        // back to whole blocks: 11.7 m / 0.5 = 23.4, rounds to 23. So blocks 0..=22
        // are ground, block 22 is the grass, and block 23 (11.5 m) is the first air
        (metres / BLOCK_SIZE as f64).round() as i32
    }

    /// The point on top of the ground above `xz` (metres)
    pub fn surface_point(&self, xz: Vec2) -> Vec3 {
        // metres -> the block column containing that point
        let x = (xz.x / BLOCK_SIZE).floor() as i32;
        let z = (xz.y / BLOCK_SIZE).floor() as i32;
        // the ground height in blocks is also where the first air block starts,
        // so that height in metres is exactly the top of the grass
        Vec3::new(xz.x, self.height_at(x, z) as f32 * BLOCK_SIZE, xz.y)
    }

    /// The block at height `y` in a column whose ground height is `height`
    fn block(&self, height: i32, y: i32) -> Block {
        // count down from the grass: the grass block is depth 0, the one under it
        // depth 1, and so on. Above the grass is negative.
        // e.g. height 23, dirt_depth 3: y 23+ air, 22 grass, 21..=19 dirt, 18- stone
        let depth = height - 1 - y;
        match depth {
            ..0 => Block::AIR,
            0 => Block::GRASS,
            d if d <= self.0.dirt_depth => Block::DIRT,
            _ => Block::STONE,
        }
    }

    /// All the blocks of one chunk. Runs on a background thread for every chunk
    /// the server needs, so it should stay fast
    pub fn generate(&self, pos: ChunkPos) -> Chunk {
        // `pos` counts in chunks; `origin` is the world block at the chunk's
        // (0, 0, 0) corner. e.g. chunk (1, -1, 0) starts at block (32, -32, 0)
        let origin = pos.0 * CHUNK_SIZE;
        let size = CHUNK_SIZE as usize;

        // Step 1: the ground height of each of the chunk's 32 x 32 columns.
        // Heights only depend on x and z, so each column is sampled once here
        // instead of once per block. Stored flat: column (x, z) is at x + z * 32
        let mut heights = vec![0; size * size];
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                heights[(x + z * CHUNK_SIZE) as usize] = self.height_at(origin.x + x, origin.z + z);
            }
        }

        // Shortcut: most chunks are entirely sky or entirely underground. Compare
        // the chunk's vertical span (world block heights `bottom..=top`) with the
        // lowest and highest ground in it:
        //
        //   bottom >= highest    every block is at or above every column's first
        //                        air block: all air
        //   top below the dirt   every block is deeper than the grass and dirt of
        //   of the lowest column even the lowest column: all stone
        //
        // Those become `Chunk::Uniform`: 1 byte, no per-block work.
        let lowest = *heights.iter().min().unwrap();
        let highest = *heights.iter().max().unwrap();
        let bottom = origin.y;
        let top = origin.y + CHUNK_SIZE - 1;
        if bottom >= highest {
            return Chunk::Uniform(Block::AIR);
        }
        // the lowest column's grass is at `lowest - 1` and its dirt goes
        // `dirt_depth` further down; anything below that is stone everywhere
        if top < lowest - 1 - self.0.dirt_depth {
            return Chunk::Uniform(Block::STONE);
        }

        // Step 2: the chunk crosses the surface somewhere, so fill it column by
        // column. `Chunk::default()` is all air, so air blocks need no work
        let mut chunk = Chunk::default();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let height = heights[(x + z * CHUNK_SIZE) as usize];
                for y in 0..CHUNK_SIZE {
                    // `y` is local to the chunk; `bottom + y` is the world height
                    let block = self.block(height, bottom + y);
                    if block != Block::AIR {
                        chunk.set(IVec3::new(x, y, z), block);
                    }
                }
            }
        }
        chunk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chunk positions from sky to deep underground, spread over the map
    fn sample_chunks() -> impl Iterator<Item = ChunkPos> {
        (-6..=4).flat_map(|y| {
            [
                IVec3::new(0, y, 0),
                IVec3::new(-7, y, 3),
                IVec3::new(40, y, -25),
            ]
            .map(ChunkPos)
        })
    }

    #[test]
    fn same_seed_same_world() {
        let settings = WorldGenSettings::default();
        let (a, b) = (Terrain::new(&settings), Terrain::new(&settings));
        for pos in sample_chunks() {
            assert_eq!(a.generate(pos), b.generate(pos), "{:?}", pos.0);
        }
        let other = Terrain::new(&WorldGenSettings {
            seed: 1,
            ..settings
        });
        assert_ne!(
            (-50..50).map(|x| a.height_at(x, 0)).collect::<Vec<_>>(),
            (-50..50).map(|x| other.height_at(x, 0)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn shortcuts_match_block_by_block() {
        let terrain = Terrain::new(&WorldGenSettings::default());
        for pos in sample_chunks() {
            let chunk = terrain.generate(pos);
            let origin = pos.0 * CHUNK_SIZE;
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let height = terrain.height_at(origin.x + x, origin.z + z);
                    for y in 0..CHUNK_SIZE {
                        let expected = terrain.block(height, origin.y + y);
                        assert_eq!(chunk.get(IVec3::new(x, y, z)), expected);
                    }
                }
            }
        }
    }

    #[test]
    fn columns_are_grass_then_dirt_then_stone() {
        let terrain = Terrain::new(&WorldGenSettings::default());
        let h = 10;
        assert_eq!(terrain.block(h, h), Block::AIR);
        assert_eq!(terrain.block(h, h - 1), Block::GRASS);
        assert_eq!(terrain.block(h, h - 4), Block::DIRT);
        assert_eq!(terrain.block(h, h - 5), Block::STONE);
    }
}
