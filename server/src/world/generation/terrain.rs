//! Turns settings and a chunk position into blocks: the code to edit to change
//! how the world is shaped. `settings.rs` holds the numbers.
//!
//! # How the ground is made
//!
//! 1. **Continentalness per column.** A very wide noise decides, for every (x, z)
//!    column, how far inland it is: -1 deep ocean, 1 far inland. The
//!    `Continents::shape` curve turns that into a base height.
//! 2. **Hills on top.** Every noise layer adds its push up or down. `inland`
//!    layers are faded by how far inland the column is, so they vanish at the
//!    coast and sea floors stay smooth.
//! 3. **Water.** Each column's water surface comes from `lakes.rs`: basins fill
//!    to their lowest rim point, and anything out at sea fills to `sea_level`.
//! 4. **Blocks from height.** Going down a column from the ground: one grass
//!    block, `dirt_depth` dirt blocks, then stone forever. Above the ground is
//!    water up to the water surface (if any), then air. Ground under water gets
//!    sand instead of grass and dirt.
//!
//! Noise is deterministic: the same seed and position always give the same value,
//! so chunks can be generated in any order, on any thread, and still line up at
//! their borders.
//!
//! Only height per column exists for now, so there are no overhangs or caves;
//! those need 3D noise, which comes later.
//!
//! # Fractal noise (fBm)
//!
//! Perlin noise is a smooth random surface between -1 and 1. fBm adds several
//! copies ("octaves"), each smaller and weaker than the last:
//!
//! The `noise` crate does the octave sum (`Fbm<Perlin>`). Continentalness uses it
//! as is; a noise layer scales it by `height` metres.
//!
//! Adjust settings.rs, but the gist is:
//! -1 Continentalness: deep ocean (deep in the negatives in height)
//! 0 Continentalness: coast (0m in height)
//! +1 Continentalness: deep inland (high in the positives in height)

use std::{f32::consts::TAU, sync::Arc};

use amity_protocol::world::{
    block::Block,
    chunk::{Chunk, ChunkPos},
    config::{BLOCK_SIZE, CHUNK_SIZE},
};
use bevy::prelude::*;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use super::{
    lakes::{CellSample, Lakes},
    settings::{Fractal, WorldGenSettings},
};

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
    /// blocks, see `WorldGenSettings::dirt_depth`
    dirt_depth: i32,
    /// where water collects on land, see `lakes.rs`
    lakes: Lakes,
    /// continentalness, -1..1
    continents: Fbm<Perlin>,
    /// see `Continents::shape`
    shape: Vec<(f64, f64)>,
    /// see `Continents::coast` and `Continents::inland`
    coast: f64,
    inland: f64,
    layers: Vec<Layer>,
}

/// A `NoiseLayer` with its noise built and ready to sample
struct Layer {
    /// returns roughly -1..1 for any point
    noise: Fbm<Perlin>,
    /// metres: what the -1..1 is multiplied by
    height: f64,
    /// faded out toward the coast, see `NoiseLayer::inland`
    inland: bool,
    /// metres across its biggest bumps. Layers too narrow for the lake grid to
    /// see are left out of it
    scale: f64,
}

/// What generation needs to know about one block column
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Column {
    /// Ground height in blocks: the y of the first block above the ground
    pub height: i32,
    /// The y of the first block above any water. Blocks from `height` up to (not
    /// including) `water` are water; `water == height` means no water here
    pub water: i32,
    /// Ground under water: topped with sand instead of grass
    pub sea_floor: bool,
}

/// Builds the `noise` crate's fBm from our `Fractal` settings
fn fractal(settings: &Fractal, seed: u32) -> Fbm<Perlin> {
    // `new(seed)` makes the random tables the noise is built from. The `set_*`
    // calls are the knobs from `Fractal`
    Fbm::<Perlin>::new(seed)
        .set_octaves(settings.octaves)
        // Perlin noise has about one bump per 1.0 of input. Multiplying positions
        // by 1 / scale means the input only moves 1.0 every `scale` metres, so
        // bumps come out `scale` metres wide
        .set_frequency(1.0 / settings.scale)
        .set_persistence(settings.persistence)
        .set_lacunarity(settings.lacunarity)
}

/// Reads a curve given as sorted (x, y) points, joining them with straight
/// lines. Past either end it stays at that end's y.
/// e.g. points (0, 0) and (1, 10): x = 0.25 gives 2.5, x = 5 gives 10
fn curve(points: &[(f64, f64)], x: f64) -> f64 {
    let (first, last) = (points[0], points[points.len() - 1]);
    if x <= first.0 {
        return first.1;
    }
    if x >= last.0 {
        return last.1;
    }
    // the two points either side of x, and how far x is from the left one (0..1)
    let right = points.iter().position(|point| point.0 >= x).unwrap();
    let ((x0, y0), (x1, y1)) = (points[right - 1], points[right]);
    let t = (x - x0) / (x1 - x0);
    y0 + (y1 - y0) * t
}

/// 0 at `from`, 1 at `to`, easing in and out between (an S-curve, so hills don't
/// start with a visible crease at the coast)
fn smoothstep(from: f64, to: f64, x: f64) -> f64 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Terrain {
    pub fn new(settings: &WorldGenSettings) -> Self {
        let layers = settings
            .layers
            .iter()
            .enumerate()
            .map(|(i, layer)| Layer {
                // each noise gets its own seed (continents: seed, layers: seed + 1,
                // seed + 2, ...): with the same seed, their bumps would line up
                noise: fractal(&layer.noise, settings.seed.wrapping_add(i as u32 + 1)),
                height: layer.height,
                inland: layer.inland,
                scale: layer.noise.scale,
            })
            .collect();
        let continents = &settings.continents;
        Self(Arc::new(TerrainInner {
            dirt_depth: settings.dirt_depth,
            lakes: Lakes::new(&settings.lakes, settings.sea_level),
            continents: fractal(&continents.noise, settings.seed),
            shape: continents.shape.clone(),
            coast: continents.coast,
            inland: continents.inland,
            layers,
        }))
    }

    /// Continentalness at (x, z) metres: -1 deep ocean .. 1 far inland
    fn continentalness(&self, x: f64, z: f64) -> f64 {
        self.0.continents.get([x, z])
    }

    /// Ground height (metres) at (x, z) metres, and the continentalness there.
    /// `coarse` leaves out layers too narrow for the lake grid (see `LakeSettings::cell`)
    fn ground(&self, x: f64, z: f64, coarse: bool) -> (f64, f64) {
        // how far inland this column is, and the base height that gives.
        // e.g. continentalness 0.1 is lowland: base 4 m
        let continentalness = self.continentalness(x, z);
        let base = curve(&self.0.shape, continentalness);

        // how much of the `inland` layers applies here: 0 at the coast (and at
        // sea), rising to 1 at `inland`. e.g. halfway between coast and inland: 0.5
        let land = smoothstep(self.0.coast, self.0.inland, continentalness);

        // every layer's push up or down, added onto the base.
        // e.g. base 4 + hills 12.3 * land 0.5 + detail -0.6 = 9.55 m
        let too_narrow = 4.0 * self.0.lakes.cell_size();
        let pushes: f64 = self
            .0
            .layers
            .iter()
            .filter(|layer| !(coarse && layer.scale < too_narrow))
            .map(|layer| {
                let push = layer.noise.get([x, z]) * layer.height;
                if layer.inland { push * land } else { push }
            })
            .sum();
        (base + pushes, continentalness)
    }

    /// What the lake grid needs at (x, z) metres. The height is a lower bound for
    /// the ground near here: the layers too narrow for the grid are left out, and
    /// the most they could lower the ground (their full `height`) is taken off
    fn cell_sample(&self, x: f64, z: f64) -> CellSample {
        let (height, continentalness) = self.ground(x, z, true);
        let too_narrow = 4.0 * self.0.lakes.cell_size();
        let narrow_dip: f64 = self
            .0
            .layers
            .iter()
            .filter(|layer| layer.scale < too_narrow)
            .map(|layer| layer.height)
            .sum();
        CellSample {
            height: (height - narrow_dip) as f32,
            ocean: continentalness < self.0.coast,
        }
    }

    /// The column of blocks at block `x`, `z`. Sampled at the column's centre
    pub fn column_at(&self, x: i32, z: i32) -> Column {
        // Noise works in metres, so settings mean the same thing whatever the
        // block size. Block 3 spans 1.5..2.0 m; its centre is (3 + 0.5) * 0.5 = 1.75 m
        let x = (x as f64 + 0.5) * BLOCK_SIZE as f64;
        let z = (z as f64 + 0.5) * BLOCK_SIZE as f64;

        let (metres, _) = self.ground(x, z, false);
        // back to whole blocks: 9.55 m / 0.5 = 19.1, rounds to 19. So blocks up to
        // 18 are ground, 18 is the top one, and 19 (9.5 m) is the first air
        let height = (metres / BLOCK_SIZE as f64).round() as i32;

        // the water surface here: its lake cell's level (see `Lakes::level_at`).
        // The ocean is part of the lake grid too, at sea level
        let level = self.0.lakes.level_at(x, z, &|x, z| self.cell_sample(x, z));
        // e.g. level 3.2 m / 0.5 = 6.4, rounds to 6: water in blocks below 6. On
        // ground above that (height >= 6), there's no water: `water` = `height`
        let water = ((level as f64 / BLOCK_SIZE as f64).round() as i32).max(height);

        Column {
            height,
            water,
            sea_floor: water > height,
        }
    }

    /// The point on top of the ground above `xz` (metres)
    pub fn surface_point(&self, xz: Vec2) -> Vec3 {
        // metres -> the block column containing that point
        let x = (xz.x / BLOCK_SIZE).floor() as i32;
        let z = (xz.y / BLOCK_SIZE).floor() as i32;
        // the ground height in blocks is also where the first air block starts,
        // so that height in metres is exactly the top of the ground
        Vec3::new(xz.x, self.column_at(x, z).height as f32 * BLOCK_SIZE, xz.y)
    }

    /// The nearest point to `from` (metres) that is dry land, searching outward in
    /// rings 64 m apart. Used for spawn, which shouldn't be at the bottom of an
    /// ocean. Only if there's no land within 50 km does it give up and use `from`
    pub fn nearest_land(&self, from: Vec2) -> Vec2 {
        // dry: no water standing on the ground
        let on_land = |xz: Vec2| {
            let x = (xz.x / BLOCK_SIZE).floor() as i32;
            let z = (xz.y / BLOCK_SIZE).floor() as i32;
            let column = self.column_at(x, z);
            column.water == column.height
        };
        if on_land(from) {
            return from;
        }
        for ring in 1..=800 {
            let radius = ring as f32 * 64.0;
            // more points on bigger rings, so they stay about 64 m apart
            let points = (TAU * radius / 64.0) as usize;
            for i in 0..points {
                let angle = TAU * i as f32 / points as f32;
                let xz = from + Vec2::from_angle(angle) * radius;
                if on_land(xz) {
                    return xz;
                }
            }
        }
        from
    }

    /// The block at height `y` in `column`
    fn block(&self, column: Column, y: i32) -> Block {
        // count down from the top block: it is depth 0, the one under it depth 1,
        // and so on. Above the ground is negative.
        // e.g. height 19, dirt_depth 3: y 19+ air, 18 grass, 17..=15 dirt, 14- stone
        let depth = column.height - 1 - y;
        match depth {
            // above the ground: water up to the column's water surface, then air.
            // e.g. height -6, water 0: blocks -6..=-1 are water, 0 and up air
            ..0 if y < column.water => Block::WATER,
            ..0 => Block::AIR,
            // under water: sand where land would have grass and dirt
            d if column.sea_floor && d <= self.0.dirt_depth => Block::SAND,
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

        // Step 1: each of the chunk's 32 x 32 columns. Columns only depend on x
        // and z, so each is sampled once here instead of once per block. Stored
        // flat: column (x, z) is at x + z * 32
        let mut columns = Vec::with_capacity(size * size);
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                columns.push(self.column_at(origin.x + x, origin.z + z));
            }
        }

        // Shortcut: most chunks are entirely sky, water or underground. Compare
        // the chunk's vertical span (world block heights `bottom..=top`) with the
        // lowest and highest ground in it, and the water surfaces:
        //
        //   bottom >= every      nothing but air: the chunk starts above all
        //   ground and water     ground and all water
        //   bottom >= highest    no ground in the chunk, and it ends below every
        //   top < lowest water   column's water surface: all water
        //   top below the dirt   every block is deeper than the grass and dirt (or
        //   of the lowest column sand) of even the lowest column: all stone
        //
        // Those become `Chunk::Uniform`: 1 byte, no per-block work.
        let lowest = columns.iter().map(|column| column.height).min().unwrap();
        let highest = columns.iter().map(|column| column.height).max().unwrap();
        // `water` is never below `height`, so the highest water is also the top of
        // everything that isn't air
        let highest_water = columns.iter().map(|column| column.water).max().unwrap();
        let lowest_water = columns.iter().map(|column| column.water).min().unwrap();
        let bottom = origin.y;
        let top = origin.y + CHUNK_SIZE - 1;
        if bottom >= highest_water {
            return Chunk::Uniform(Block::AIR);
        }
        if bottom >= highest && top < lowest_water {
            return Chunk::Uniform(Block::WATER);
        }
        // the lowest column's top block is at `lowest - 1` and its dirt goes
        // `dirt_depth` further down; anything below that is stone everywhere
        if top < lowest - 1 - self.0.dirt_depth {
            return Chunk::Uniform(Block::STONE);
        }

        // Step 2: the chunk crosses the ground or the sea's surface somewhere, so
        // fill it column by column. `Chunk::default()` is all air, so air blocks
        // need no work
        let mut chunk = Chunk::default();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let column = columns[(x + z * CHUNK_SIZE) as usize];
                for y in 0..CHUNK_SIZE {
                    // `y` is local to the chunk; `bottom + y` is the world height
                    let block = self.block(column, bottom + y);
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
            (-50..50).map(|x| a.column_at(x, 0)).collect::<Vec<_>>(),
            (-50..50).map(|x| other.column_at(x, 0)).collect::<Vec<_>>()
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
                    let column = terrain.column_at(origin.x + x, origin.z + z);
                    for y in 0..CHUNK_SIZE {
                        let expected = terrain.block(column, origin.y + y);
                        assert_eq!(chunk.get(IVec3::new(x, y, z)), expected);
                    }
                }
            }
        }
    }

    #[test]
    fn columns_are_grass_then_dirt_then_stone() {
        let terrain = Terrain::new(&WorldGenSettings::default());
        let land = Column {
            height: 10,
            water: 10,
            sea_floor: false,
        };
        assert_eq!(terrain.block(land, 10), Block::AIR);
        assert_eq!(terrain.block(land, 9), Block::GRASS);
        assert_eq!(terrain.block(land, 6), Block::DIRT);
        assert_eq!(terrain.block(land, 5), Block::STONE);

        let sea = Column {
            water: 12,
            sea_floor: true,
            ..land
        };
        assert_eq!(terrain.block(sea, 9), Block::SAND);
        assert_eq!(terrain.block(sea, 6), Block::SAND);
        assert_eq!(terrain.block(sea, 5), Block::STONE);
    }

    #[test]
    fn curve_joins_points_and_holds_its_ends() {
        let points = [(-1.0, -10.0), (0.0, 0.0), (1.0, 30.0)];
        assert_eq!(curve(&points, -5.0), -10.0);
        assert_eq!(curve(&points, -0.5), -5.0);
        assert_eq!(curve(&points, 0.5), 15.0);
        assert_eq!(curve(&points, 5.0), 30.0);
    }

    #[test]
    fn spawn_search_finds_land() {
        let terrain = Terrain::new(&WorldGenSettings::default());
        let land = terrain.nearest_land(Vec2::ZERO);
        assert!(terrain.surface_point(land).y > 0.0);
    }

    #[test]
    fn oceans_fill_to_sea_level() {
        let terrain = Terrain::new(&WorldGenSettings::default());
        let deep = Column {
            height: -10,
            water: 0,
            sea_floor: true,
        };
        assert_eq!(terrain.block(deep, 0), Block::AIR);
        assert_eq!(terrain.block(deep, -1), Block::WATER);
        assert_eq!(terrain.block(deep, -10), Block::WATER);
        assert_eq!(terrain.block(deep, -11), Block::SAND);

        // a world that is all ocean, 40 m deep: whole chunks of water and of air
        let mut settings = WorldGenSettings::default();
        settings.continents.shape = vec![(-1.0, -40.0), (1.0, -40.0)];
        // every continentalness is below the coast: all of it is ocean
        settings.continents.coast = 2.0;
        settings.continents.inland = 3.0;
        settings.layers.clear();
        let ocean = Terrain::new(&settings);
        assert_eq!(
            ocean.generate(ChunkPos(IVec3::new(3, -1, 3))),
            Chunk::Uniform(Block::WATER)
        );
        assert_eq!(
            ocean.generate(ChunkPos(IVec3::new(3, 0, 3))),
            Chunk::Uniform(Block::AIR)
        );
    }

    #[test]
    fn no_water_hangs_over_a_neighbour() {
        // every pair of neighbouring columns around spawn (on the coast), and
        // across a region border (block x 8192) where a lake once got a different
        // level on each side: a column's top is never below the water beside it
        let terrain = Terrain::new(&WorldGenSettings::default());
        let spawn = terrain.nearest_land(Vec2::ZERO);
        let spawn = ((spawn.x / BLOCK_SIZE) as i32, (spawn.y / BLOCK_SIZE) as i32);
        for (cx, cz) in [spawn, (8192, 8256)] {
            for z in cz - 64..cz + 64 {
                for x in cx - 64..cx + 64 {
                    let here = terrain.column_at(x, z);
                    for there in [terrain.column_at(x + 1, z), terrain.column_at(x, z + 1)] {
                        for (a, b) in [(here, there), (there, here)] {
                            if b.water > b.height {
                                assert!(a.water.max(a.height) >= b.water, "{a:?} beside {b:?}");
                            }
                        }
                    }
                }
            }
        }
    }
}
