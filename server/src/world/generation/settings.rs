//! What the generated world looks like: the numbers to tweak. `terrain.rs`
//! explains how they are used.
//!
//! Units: heights and sizes are in metres (a block is BLOCK_SIZE = 0.5 m),
//! depths counted in blocks say so.

use bevy::prelude::*;

/// One world's generation settings. Later this can be loaded from a preset file,
/// one per world type
#[derive(Resource, Clone, Debug)]
pub struct WorldGenSettings {
    /// Picks which world you get. Same seed + same settings = the same world,
    /// block for block, every time
    pub seed: u32,
    /// Blocks of dirt between the grass and the stone (and of sand under water)
    pub dirt_depth: i32,
    /// Height of the ocean's surface (metres). Ocean, and basins connected to it,
    /// fill with water up to here
    pub sea_level: f64,
    /// Lakes: basins on land fill up to their lowest rim point. See `LakeSettings`
    pub lakes: LakeSettings,
    /// Where land and ocean are, and the base height of each. See `Continents`
    pub continents: Continents,
    /// Noise layers added on top of the continents' base height. See `NoiseLayer`
    pub layers: Vec<NoiseLayer>,
}

impl Default for WorldGenSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            dirt_depth: 3,
            sea_level: 0.0,
            lakes: LakeSettings {
                region: 2048.0,
                margin: 1024.0,
                cell: 8.0,
                min_depth: 1.0,
            },
            continents: Continents {
                noise: Fractal {
                    scale: 3000.0,
                    octaves: 4,
                    persistence: 0.5,
                    lacunarity: 2.0,
                },
                shape: vec![
                    (-1.0, -60.0), // deep ocean
                    (-0.4, -40.0),
                    (-0.15, -15.0), // continental shelf
                    (-0.05, -1.0),  // coast
                    (0.1, 4.0),     // lowland
                    (1.0, 30.0),    // far inland
                ],
                coast: -0.05,
                inland: 0.2,
            },
            layers: vec![
                // the rolling shape of the land
                NoiseLayer {
                    noise: Fractal {
                        scale: 250.0,
                        octaves: 5,
                        persistence: 0.6,
                        lacunarity: 2.0,
                    },
                    height: 64.0,
                    inland: true,
                },
                // small lumps you can walk over
                NoiseLayer {
                    noise: Fractal {
                        scale: 30.0,
                        octaves: 2,
                        persistence: 0.5,
                        lacunarity: 2.0,
                    },
                    height: 5.0,
                    inland: false,
                },
            ],
        }
    }
}

/// How lakes are found. `lakes.rs` explains the method: the world is split into
/// square regions, and each is "flooded" on a coarse grid to find where water
/// would collect.
#[derive(Clone, Debug)]
pub struct LakeSettings {
    /// Metres along a region's edge. Mostly a performance knob: bigger regions are
    /// computed less often but take longer each time
    pub region: f64,
    /// Metres of extra ground flooded around each region, so lakes near its edge
    /// see their whole rim. Lakes wider than this, lying across a region border,
    /// can get a different water level on each side. Raise it if you see a step
    /// in a lake's surface; it costs more time per region
    pub margin: f64,
    /// Metres per grid cell. Smaller finds smaller lakes and follows the terrain
    /// more closely, but costs a lot more (halving it is 4x the work and memory).
    /// Noise layers narrower than 4 cells are left out of the grid: they'd only
    /// make puddles
    pub cell: f64,
    /// Metres: lakes shallower than this at their deepest are dropped
    pub min_depth: f64,
}

/// Continents and oceans. Their noise, "continentalness", is a very wide
/// fractal noise from -1 (deep ocean) to 1 (far inland). `shape` turns it into
/// the ground's base height, and the `NoiseLayer`s add hills on top.
#[derive(Clone, Debug)]
pub struct Continents {
    /// The continentalness noise. Its `scale` is roughly how far apart continents
    /// and oceans are: raise for bigger continents, lower for more, smaller ones
    pub noise: Fractal,
    /// Continentalness -> base height (metres), as points on a curve, sorted by
    /// continentalness. Heights between points are joined by straight lines.
    ///
    /// This is where coastlines are shaped: points close together make a steep
    /// change (a cliff-like shelf drop), points far apart a gentle one (long
    /// flat lowlands). Islands appear wherever the noise pokes above the coast
    /// point out in an ocean
    pub shape: Vec<(f64, f64)>,
    /// Continentalness of the shoreline, where `inland` layers start fading in.
    /// Should match where `shape` crosses sea level
    pub coast: f64,
    /// Continentalness where `inland` layers reach full strength. They fade in
    /// from nothing at `coast`, so sea floors stay smooth and hills rise away
    /// from the shore. Closer to `coast` means hills start right at the beach
    pub inland: f64,
}

/// One layer of fractal noise (fBm) that raises and lowers the ground.
#[derive(Clone, Debug)]
pub struct NoiseLayer {
    /// The shape of the layer's bumps
    pub noise: Fractal,
    /// Metres the layer moves the ground up or down, at most (most places move
    /// much less). Raise for taller hills and deeper valleys
    pub height: f64,
    /// true: fades out toward the coast and is absent at sea (hills, mountains).
    /// false: applies everywhere, sea floors included (small detail)
    pub inland: bool,
}

/// Fractal noise (fBm): several copies ("octaves") of smooth Perlin noise added
/// up. The first is `scale` wide; each next octave is `lacunarity` times smaller
/// and `persistence` times weaker. Big shapes with smaller bumps on top, like real
/// land. Its value is between -1 and 1, mostly within about ±0.5
#[derive(Clone, Debug)]
pub struct Fractal {
    /// Metres across the biggest features (roughly hill to hill). Raise for wider,
    /// gentler land; lower for tighter, busier land
    pub scale: f64,
    /// How many octaves are stacked. Each adds finer detail, and costs a little
    /// generation time. 1 is plain smooth Perlin noise
    pub octaves: usize,
    /// 0..1: how strong each octave is compared with the one before. Low (0.3) is
    /// smooth, high (0.7) is rough and jagged
    pub persistence: f64,
    /// How much smaller each octave's features are than the one before. 2.0 halves
    /// them each time; rarely worth changing
    pub lacunarity: f64,
}
