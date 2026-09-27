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
    /// Where the land is flat and where it's mountainous. See `Relief`
    pub relief: Relief,
    /// Noise layers added on top of the continents' base height. See `NoiseLayer`
    pub layers: Vec<NoiseLayer>,
    /// Gullies and ridges carved into the hills, like running water would. See
    /// `ErosionSettings`
    pub erosion: ErosionSettings,
    /// What the top of the ground is made of. See `SurfaceSettings`
    pub surface: SurfaceSettings,
    /// Temperature and rainfall everywhere. See `Climate`
    pub climate: Climate,
    /// The kinds of land, picked by climate. Checked top to bottom: the first
    /// whose temperature and rainfall ranges hold wins (the last if none does).
    /// See `Biome`
    pub biomes: Vec<Biome>,
}

impl Default for WorldGenSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            sea_level: 0.0,
            lakes: LakeSettings {
                region: 2048.0,
                margin: 512.0,
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
            relief: Relief {
                noise: Fractal {
                    scale: 500.0,
                    octaves: 3,
                    persistence: 0.5,
                    lacunarity: 2.0,
                },
                shape: vec![
                    (-1.0, 0.05), // plains
                    (-0.1, 0.1),
                    (0.1, 0.4),  // rolling hills
                    (0.35, 1.0), // mountains
                    (1.0, 1.3),
                ],
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
            erosion: ErosionSettings {
                enabled: true,
                scale: 100.0,
                strength: 0.15,
                gully_weight: 0.5,
                detail: 1.5,
                rounding: [0.1, 0.0, 0.1, 2.0],
                onset: [1.25, 1.25, 2.8, 1.5],
                assumed_slope: [0.7, 1.0],
                cell_scale: 0.7,
                octaves: 5,
                gain: 0.5,
                lacunarity: 2.0,
                normalization: 0.5,
                height_offset: [0.0, 0.0],
                fade_height: 84.0,
                inland: true,
                follow_relief: true,
            },
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
                    // anything else gets its biome's `ground`
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
            climate: Climate {
                temperature: ClimateNoise {
                    noise: Fractal {
                        scale: 4000.0,
                        octaves: 3,
                        persistence: 0.5,
                        lacunarity: 2.0,
                    },
                    range: (-15.0, 32.0),
                },
                rainfall: ClimateNoise {
                    noise: Fractal {
                        scale: 3000.0,
                        octaves: 3,
                        persistence: 0.5,
                        lacunarity: 2.0,
                    },
                    range: (0.0, 400.0),
                },
                lapse_rate: 0.65,
                jitter: Jitter {
                    noise: Fractal {
                        scale: 40.0,
                        octaves: 2,
                        persistence: 0.5,
                        lacunarity: 2.0,
                    },
                    temperature: 1.5,
                    rainfall: 15.0,
                },
            },
            // a simplified Whittaker diagram: biomes by temperature and rainfall
            biomes: vec![
                Biome {
                    name: "tundra",
                    temperature: (f64::MIN, 0.0),
                    rainfall: ANY,
                    rules: vec![],
                    ground: vec![(Block::SNOW, 1), (Block::GRAVEL, 2)],
                },
                Biome {
                    name: "taiga",
                    temperature: (0.0, 6.0),
                    rainfall: ANY,
                    rules: vec![],
                    ground: vec![(Block::PODZOL, 1), (Block::DIRT, 3)],
                },
                Biome {
                    name: "grassland",
                    temperature: (6.0, 20.0),
                    rainfall: (f64::MIN, 100.0),
                    rules: vec![],
                    ground: vec![(Block::DRY_GRASS, 1), (Block::DIRT, 3)],
                },
                Biome {
                    name: "temperate forest",
                    temperature: (6.0, 20.0),
                    rainfall: ANY,
                    rules: vec![],
                    ground: vec![(Block::GRASS, 1), (Block::DIRT, 3)],
                },
                Biome {
                    name: "desert",
                    temperature: (20.0, f64::MAX),
                    rainfall: (f64::MIN, 60.0),
                    rules: vec![
                        // sand, not gravel, on the dunes' steep sides
                        SurfaceRule {
                            when: When {
                                slope: (35.0, 50.0),
                                ..When::default()
                            },
                            layers: vec![(Block::SAND, 2), (Block::SANDSTONE, 4)],
                        },
                    ],
                    ground: vec![(Block::SAND, 4), (Block::SANDSTONE, 6)],
                },
                Biome {
                    name: "savanna",
                    temperature: (20.0, f64::MAX),
                    rainfall: (60.0, 180.0),
                    rules: vec![],
                    ground: vec![(Block::DRY_GRASS, 1), (Block::DIRT, 3)],
                },
                Biome {
                    name: "tropical forest",
                    temperature: (20.0, f64::MAX),
                    rainfall: ANY,
                    rules: vec![],
                    ground: vec![(Block::GRASS, 1), (Block::DIRT, 3)],
                },
            ],
        }
    }
}

/// Any value: a range for `Biome` conditions that always holds
pub const ANY: (f64, f64) = (f64::MIN, f64::MAX);

/// Temperature and rainfall for every column. Each is a wide noise mapped onto a
/// range; `terrain.rs` explains how biomes use them
#[derive(Clone, Debug)]
pub struct Climate {
    /// Degrees Celsius, at sea level
    pub temperature: ClimateNoise,
    /// Centimetres per year
    pub rainfall: ClimateNoise,
    /// Degrees Celsius colder per 100 m of altitude. Real air: about 0.65. Our
    /// hills are low, so raise it to get cold mountain tops
    pub lapse_rate: f64,
    /// Small noise added to both, so biome borders wander instead of following
    /// the smooth climate noise's contour lines
    pub jitter: Jitter,
}

/// One climate value: a wide noise spread over a range
#[derive(Clone, Debug)]
pub struct ClimateNoise {
    /// Its `scale` is roughly how far apart hot and cold (or wet and dry) places
    /// are
    pub noise: Fractal,
    /// (lowest, highest). The noise is mostly within ±0.5, which is mapped onto
    /// this range; the rare values beyond go past its ends
    pub range: (f64, f64),
}

/// Ragged biome borders
#[derive(Clone, Debug)]
pub struct Jitter {
    /// Its `scale` is how wiggly borders are
    pub noise: Fractal,
    /// Degrees Celsius it moves the temperature, at most
    pub temperature: f64,
    /// Centimetres per year it moves the rainfall, at most
    pub rainfall: f64,
}

/// A kind of land: where it is (by climate) and what its ground is made of
#[derive(Clone, Debug)]
pub struct Biome {
    /// For reading the settings and debugging
    pub name: &'static str,
    /// (min, max) degrees Celsius; min included, max not. `ANY` for any
    pub temperature: (f64, f64),
    /// (min, max) centimetres per year; min included, max not. `ANY` for any
    pub rainfall: (f64, f64),
    /// Surface rules for this biome only, checked before the global ones in
    /// `SurfaceSettings::rules`
    pub rules: Vec<SurfaceRule>,
    /// The layers for any column no rule matches: this biome's normal ground
    pub ground: Vec<(Block, i32)>,
}

/// The erosion filter (`erosion.rs` explains it). The defaults are the
/// filter's author's, with sizes turned into metres for hills about 250 m wide.
///
/// Erosion follows the slope of the big shapes only: the continents plus the
/// noise layers at least 4 lake cells wide (`LakeSettings::cell`). Narrower layers
/// are small detail, added on top afterwards
#[derive(Clone, Debug)]
pub struct ErosionSettings {
    /// false: no erosion at all (and no cost)
    pub enabled: bool,
    /// Metres: the size of the biggest gullies. Bigger spreads them further apart
    /// *and* cuts them deeper by the same amount, so the land looks the same, just
    /// bigger: it doesn't change how steep erosion makes things
    pub scale: f64,
    /// How deep gullies cut, relative to `scale`: this sets how steep eroded land
    /// gets. The biggest octave moves the ground up to `strength * scale` metres;
    /// all together about twice that. The filter's author uses 0.22, which makes
    /// ~60 degree gully walls: too spiky for half-metre blocks
    pub strength: f64,
    /// 0..1: gullies versus sharp peaks. Higher: deeper gullies, rounder peaks
    pub gully_weight: f64,
    /// How much smaller gullies can cut into the slopes of bigger ones. Higher:
    /// more fine detail on steep ground
    pub detail: f64,
    /// Rounding of ridges and creases: (ridges, creases, of the input terrain,
    /// growth per octave). Higher values give softer, rounder shapes
    pub rounding: [f64; 4],
    /// How steep the ground must be before gullies appear: (initial slope, each
    /// octave's slope, ridge map initial, ridge map per octave). Higher: gullies
    /// start on gentler slopes
    pub onset: [f64; 4],
    /// (assumed steepness, how much to use it instead of the real one). Makes
    /// gullies form evenly on gentle and steep ground alike
    pub assumed_slope: [f64; 2],
    /// Size of the random cells gullies are laid out in, relative to `scale`.
    /// Smaller: more, shorter, wigglier gullies
    pub cell_scale: f64,
    /// How many sizes of gullies. Each is smaller and weaker than the last
    pub octaves: usize,
    /// How much weaker each octave is than the one before
    pub gain: f64,
    /// How much smaller each octave is than the one before
    pub lacunarity: f64,
    /// 0..1: how much to restore gully depth where cells blend and waves cancel
    pub normalization: f64,
    /// Lowers the terrain by a share of the erosion's total strength: (share, how
    /// much more on peaks than valleys, 0..1). The filter's author uses -0.65, for
    /// terrain whose heights are all above its average; ours go both ways around
    /// the continents' base, so 0 keeps the land at the same height on average
    pub height_offset: [f64; 2],
    /// Metres: how tall the hills are, used to tell valleys (-1) from peaks (1)
    /// so erosion keeps their shape. Match the main hill layer's `height`
    pub fade_height: f64,
    /// true: fades out towards the coast like `inland` noise layers, so beaches
    /// and sea floors stay smooth
    pub inland: bool,
    /// true: scaled by the relief multiplier (see `Relief`), up to 1: full
    /// strength in the mountains, faint on plains and gentle hills
    pub follow_relief: bool,
}

/// What the top of the ground is made of. `surface.rs` explains how the rules
/// are used
#[derive(Clone, Debug)]
pub struct SurfaceSettings {
    /// Checked top to bottom for every column, after its biome's own rules: the
    /// first rule whose `when` matches gives the column its layers. A column no
    /// rule matches gets its biome's `ground`
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

/// What kind of land a place is, from flat plains to mountains. A very wide
/// noise, like continentalness, whose value `shape` turns into how much of the
/// `inland` noise layers applies there: near 0 is plains, 1 is the layers at their
/// full `height`. Erosion needs slope, so plains stay smooth by themselves
#[derive(Clone, Debug)]
pub struct Relief {
    /// Its `scale` is roughly how big each plain or mountain range is
    pub noise: Fractal,
    /// Relief noise (-1..1, mostly within about ±0.5) -> multiplier for the
    /// `inland` layers, as points on a curve, sorted by the noise value. Heights
    /// between points are joined by straight lines. Spread the low points wider
    /// for more plains, the high ones for more mountains
    pub shape: Vec<(f64, f64)>,
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
