//! What the generated world looks like: the numbers to tweak. `terrain.rs`
//! explains how they are used.
//!
//! Units: heights and sizes are in metres (a block is BLOCK_SIZE = 0.5 m),
//! depths counted in blocks say so.

use amity_protocol::world::block::Block;
use bevy::prelude::*;

/// One world's generation settings. Later this can be loaded from a preset file,
/// one per world type
#[derive(Resource, Clone, Debug)]
pub struct WorldGenSettings {
    /// Picks which world you get. Same seed + same settings = the same world,
    /// block for block, every time
    pub seed: u32,
    /// Height of the ocean's surface (metres). Ocean, and basins connected to it,
    /// fill with water up to here
    pub sea_level: f64,
    /// Lakes: basins on land fill up to their lowest rim point. See `LakeSettings`
    pub lakes: LakeSettings,
    /// Where land and ocean are, and the base height of each. See `Continents`
    pub continents: Continents,
    /// Noise layers added on top of the continents' base height. See `NoiseLayer`
    pub layers: Vec<NoiseLayer>,
    /// What the top of the ground is made of. See `SurfaceSettings`
    pub surface: SurfaceSettings,
}

impl Default for WorldGenSettings {
    fn default() -> Self {
        Self {
            seed: 0,
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
            surface: SurfaceSettings {
                rules: vec![
                    // lake and sea floors
                    SurfaceRule {
                        when: When {
                            under_water: Some(true),
                            ..When::default()
                        },
                        layers: vec![(Block::SAND, 4)],
                    },
                    // beaches: dry ground just above water
                    SurfaceRule {
                        when: When {
                            under_water: Some(false),
                            above_water: Some((-2.0, 2.0)),
                            ..Default::default()
                        },
                        layers: vec![(Block::SAND, 4)],
                    },
                    // cliffs: too steep for anything to stay, so bare stone
                    SurfaceRule {
                        when: When {
                            slope: (50.0, 90.0),
                            ..When::default()
                        },
                        layers: vec![],
                    },
                    // scree: loose rock on steep slopes
                    SurfaceRule {
                        when: When {
                            slope: (35.0, 50.0),
                            ..When::default()
                        },
                        layers: vec![(Block::GRAVEL, 2)],
                    },
                    // snow on the peaks: roughly the highest 2% of land
                    SurfaceRule {
                        when: When {
                            altitude: (40.0, f64::MAX),
                            ..When::default()
                        },
                        layers: vec![(Block::SNOW, 2)],
                    },
                    // everything else
                    SurfaceRule {
                        when: When::default(),
                        layers: vec![(Block::GRASS, 1), (Block::DIRT, 3)],
                    },
                ],
                wobble: Wobble {
                    noise: Fractal {
                        scale: 60.0,
                        octaves: 3,
                        persistence: 0.5,
                        lacunarity: 2.0,
                    },
                    height: 6.0,
                },
                shore_reach: 4,
            },
        }
    }
}

/// What the top of the ground is made of. `surface.rs` explains how the rules
/// are used
#[derive(Clone, Debug)]
pub struct SurfaceSettings {
    /// Checked top to bottom for every column: the first rule whose `when`
    /// matches gives the column its layers. A column no rule matches is bare
    /// stone, so the last rule should usually match anything
    pub rules: Vec<SurfaceRule>,
    /// Shifts the altitude the rules see by a little noise, so an altitude line
    /// like the snowline wanders across the land instead of being perfectly flat
    pub wobble: Wobble,
    /// Blocks: how far from a column `When::above_water` looks for water. Only
    /// water actually standing in a column counts, so beaches only form beside
    /// water you can see. Wider beaches, but more columns sampled per chunk
    pub shore_reach: i32,
}

/// One surface rule: where it applies, and what it puts there
#[derive(Clone, Debug)]
pub struct SurfaceRule {
    /// The conditions a column must meet. `When::default()` matches anything
    pub when: When,
    /// The blocks from the top of the ground down, as (block, how many blocks
    /// thick). Stone continues below. Empty: bare stone
    pub layers: Vec<(Block, i32)>,
}

/// Conditions on a column. Every one set must hold; unset ones don't matter.
/// Ranges are (min, max): min included, max not
#[derive(Clone, Debug)]
pub struct When {
    /// Ground height in metres (with the wobble added)
    /// Min, Max
    pub altitude: (f64, f64),
    /// Steepness in degrees: 0 flat, 45 one block up per block across, 90 a wall
    pub slope: (f64, f64),
    /// Some(true): only under water. Some(false): only dry. None: either
    pub under_water: Option<bool>,
    /// Only where water stands within `SurfaceSettings::shore_reach` blocks, and
    /// the ground is this many metres above its surface. Good for beaches and shores
    pub above_water: Option<(f64, f64)>,
}

impl Default for When {
    /// Matches anything
    fn default() -> Self {
        Self {
            altitude: (f64::MIN, f64::MAX),
            slope: (0.0, 91.0),
            under_water: None,
            above_water: None,
        }
    }
}

/// A little noise added to a height
#[derive(Clone, Debug)]
pub struct Wobble {
    /// The shape of the wobble: `scale` is how far apart its ups and downs are
    pub noise: Fractal,
    /// Metres it moves the height up or down, at most
    pub height: f64,
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
