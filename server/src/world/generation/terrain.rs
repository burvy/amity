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
//! 3. **Erosion.** Gullies and ridges are carved into the hills (see
//!    `erosion.rs`), following their slope. The narrowest layers (small detail)
//!    are added only after, so gullies follow the land's big shape.
//! 4. **Water.** Each column's water surface comes from `lakes.rs`: basins fill
//!    to their lowest rim point, and anything out at sea fills to `sea_level`.
//! 5. **Climate and biome.** Each column gets a temperature (colder higher up)
//!    and a rainfall from wide noises, and the first biome whose ranges hold them
//!    is its biome: tundra, taiga, grassland, forest, desert...
//! 6. **Surface.** The column's slope, altitude and nearby water pick a surface
//!    rule (see `surface.rs`): its biome's own rules first, then the global ones,
//!    else the biome's normal ground. The rule says what layers top the ground:
//!    grass over dirt, sand under water and in deserts, gravel on steep slopes...
//! 7. **Blocks.** Going down a column: air, then water down to the ground (if
//!    any), then the rule's layers, then stone forever.
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
use bevy::{math::DVec2, prelude::*};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use super::{
    erosion::{self, octave_bound, octave_spacing},
    lakes::{CellSample, Lakes, SampleGrid},
    settings::{Biome, ErosionSettings, Fractal, SurfaceRule, When, WorldGenSettings},
    surface::{self, Facts},
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
    /// every biome, with its complete list of surface rules
    biomes: Vec<BiomeRules>,
    /// blocks: the thickest stack of layers any rule has. Below that, every
    /// column is stone, whichever rule it follows
    deepest_layers: i32,
    /// see `Climate`: temperature and rainfall noises, and the jitter's
    temperature: Fbm<Perlin>,
    rainfall: Fbm<Perlin>,
    jitter: Fbm<Perlin>,
    climate: super::settings::Climate,
    /// see `SurfaceSettings::wobble`
    wobble: Fbm<Perlin>,
    wobble_height: f64,
    /// blocks, see `SurfaceSettings::shore_reach`
    shore_reach: i32,
    /// where water collects on land, see `lakes.rs`
    lakes: Lakes,
    /// continentalness, -1..1
    continents: Fbm<Perlin>,
    /// see `Continents::shape`
    shape: Vec<(f64, f64)>,
    /// see `Continents::coast` and `Continents::inland`
    coast: f64,
    inland: f64,
    /// see `Relief`
    relief: Fbm<Perlin>,
    relief_shape: Vec<(f64, f64)>,
    layers: Vec<Layer>,
    /// see `ErosionSettings`
    erosion: ErosionSettings,
    /// the erosion's random seed
    erosion_seed: i32,
    /// how many erosion octaves the lake grid computes: those whose gullies are
    /// at least 4 lake cells apart. The rest are bounded, like narrow layers
    coarse_octaves: usize,
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
    /// The column's biome, as its index in `WorldGenSettings::biomes`
    pub biome: usize,
    /// Which surface rule the column follows, as its index in its biome's
    /// complete rule list (see `BiomeRules`)
    pub surface: usize,
}

/// A biome as generation uses it: its climate ranges, and every surface rule a
/// column in it can follow, in the order they're checked: the biome's own rules,
/// then the global ones, then its `ground` as a rule that matches anything
struct BiomeRules {
    temperature: (f64, f64),
    rainfall: (f64, f64),
    rules: Vec<SurfaceRule>,
}

impl BiomeRules {
    fn new(biome: &Biome, global: &[SurfaceRule]) -> Self {
        let ground = SurfaceRule {
            when: When::default(),
            layers: biome.ground.clone(),
        };
        Self {
            temperature: biome.temperature,
            rainfall: biome.rainfall,
            rules: biome
                .rules
                .iter()
                .chain(global)
                .cloned()
                .chain([ground])
                .collect(),
        }
    }
}

/// A column before its surface is picked: what the noise and lakes say about it.
/// Picking the surface needs the 4 neighbouring columns' too (for the slope)
#[derive(Clone, Copy, Debug)]
struct RawColumn {
    /// ground height, metres, before rounding to blocks
    metres: f64,
    /// ground height and water top, in blocks, as in `Column`
    height: i32,
    water: i32,
}

impl RawColumn {
    /// The top of the water standing in this column, in blocks, if any
    fn water_top(&self) -> Option<i32> {
        (self.water > self.height).then_some(self.water)
    }
}

/// Metres between the points erosion is computed at. Columns between them take a
/// blend of the 4 around them: erosion is the costliest part of a column, and
/// even its smallest gullies are several metres across, so nothing shows
const EROSION_GRID: f64 = 1.0;

/// The land's big shape at a point: the continents and the wide noise layers
#[derive(Clone, Copy)]
struct Wide {
    /// metres
    height: f64,
    /// metres the wide layers pushed it up or down (the hills alone)
    relief: f64,
    /// 0 at the coast and at sea .. 1 inland, see `Continents::inland`
    land: f64,
    /// plains (near 0) .. mountains (1 and up), see `Relief`
    mountains: f64,
    continentalness: f64,
}

/// The centre of block column `x`, `z`, in metres
fn column_centre(x: i32, z: i32) -> (f64, f64) {
    (
        (x as f64 + 0.5) * BLOCK_SIZE as f64,
        (z as f64 + 0.5) * BLOCK_SIZE as f64,
    )
}

/// Blends 4 values at the corners of a square, (0,0) (1,0) (0,1) (1,1), for a
/// point `tx`, `tz` (0..1) across it
fn bilinear([a, b, c, d]: [f64; 4], tx: f64, tz: f64) -> f64 {
    let top = a + (b - a) * tx;
    let bottom = c + (d - c) * tx;
    top + (bottom - top) * tz
}

/// The 4 columns beside a column (east, west, south, north), as block offsets
const NEIGHBOURS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

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
        let surface = &settings.surface;
        let biomes: Vec<BiomeRules> = settings
            .biomes
            .iter()
            .map(|biome| BiomeRules::new(biome, &surface.rules))
            .collect();
        // seeds after the continents (seed), layers (seed + 1 ...), wobble and relief
        let climate_seed = |n: u32| settings.seed.wrapping_add(settings.layers.len() as u32 + n);
        Self(Arc::new(TerrainInner {
            deepest_layers: biomes
                .iter()
                .flat_map(|biome| &biome.rules)
                .map(|rule| rule.layers.iter().map(|(_, thick)| thick).sum())
                .max()
                .unwrap_or(0),
            biomes,
            temperature: fractal(&settings.climate.temperature.noise, climate_seed(3)),
            rainfall: fractal(&settings.climate.rainfall.noise, climate_seed(4)),
            jitter: fractal(&settings.climate.jitter.noise, climate_seed(5)),
            climate: settings.climate.clone(),
            // after the continents (seed) and every layer (seed + 1 ...)
            wobble: fractal(
                &surface.wobble.noise,
                settings.seed.wrapping_add(settings.layers.len() as u32 + 1),
            ),
            wobble_height: surface.wobble.height,
            // after the wobble
            relief: fractal(
                &settings.relief.noise,
                settings.seed.wrapping_add(settings.layers.len() as u32 + 2),
            ),
            relief_shape: settings.relief.shape.clone(),
            shore_reach: surface.shore_reach.max(0),
            lakes: Lakes::new(&settings.lakes, settings.sea_level),
            continents: fractal(&continents.noise, settings.seed),
            shape: continents.shape.clone(),
            coast: continents.coast,
            inland: continents.inland,
            layers,
            erosion: settings.erosion.clone(),
            erosion_seed: settings.seed as i32,
            coarse_octaves: (0..settings.erosion.octaves)
                .take_while(|&octave| {
                    octave_spacing(&settings.erosion, octave) >= 4.0 * settings.lakes.cell
                })
                .count(),
        }))
    }

    /// Continentalness at (x, z) metres: -1 deep ocean .. 1 far inland
    fn continentalness(&self, x: f64, z: f64) -> f64 {
        self.0.continents.get([x, z])
    }

    /// Metres across which a noise layer counts as small detail: too narrow for
    /// the lake grid to see, and added after erosion
    fn too_narrow(&self) -> f64 {
        4.0 * self.0.lakes.cell_size()
    }

    /// Metres either side of a point that erosion measures the slope over. Half a
    /// lake cell, so the lake grid's sample points (half a cell apart) find
    /// their slope neighbours among each other
    fn slope_step(&self) -> f64 {
        self.0.lakes.cell_size() / 2.0
    }

    /// The land's big shape at (x, z) metres, before erosion and detail
    fn wide_ground(&self, x: f64, z: f64) -> Wide {
        // how far inland this column is, and the base height that gives.
        // e.g. continentalness 0.1 is lowland: base 4 m
        let continentalness = self.continentalness(x, z);
        let base = curve(&self.0.shape, continentalness);

        // how much of the `inland` layers applies here: 0 at the coast (and at
        // sea), rising to 1 at `inland`. e.g. halfway between coast and inland: 0.5
        let land = smoothstep(self.0.coast, self.0.inland, continentalness);

        // plains or mountains: how much of the `inland` layers this kind of land
        // gets. e.g. 0.1 on a plain, 1 in the mountains
        let mountains = curve(&self.0.relief_shape, self.0.relief.get([x, z]));

        // every wide layer's push up or down, added onto the base.
        // e.g. base 4 + hills 12.3 * (land 0.5 * mountains 1) = 10.15 m
        let too_narrow = self.too_narrow();
        let relief: f64 = self
            .0
            .layers
            .iter()
            .filter(|layer| layer.scale >= too_narrow)
            .map(|layer| self.push(layer, x, z, land * mountains))
            .sum();
        Wide {
            height: base + relief,
            relief,
            land,
            mountains,
            continentalness,
        }
    }

    /// One layer's push up or down at (x, z) metres
    fn push(&self, layer: &Layer, x: f64, z: f64, land: f64) -> f64 {
        let push = layer.noise.get([x, z]) * layer.height;
        if layer.inland { push * land } else { push }
    }

    /// The small detail layers' push at (x, z) metres, added after erosion
    fn detail(&self, x: f64, z: f64, land: f64) -> f64 {
        let too_narrow = self.too_narrow();
        self.0
            .layers
            .iter()
            .filter(|layer| layer.scale < too_narrow)
            .map(|layer| self.push(layer, x, z, land))
            .sum()
    }

    /// Erosion (metres to add) at `p`, whose big shape is `centre`, given the big
    /// shape's height `slope_step` metres east, west, south and north of it
    fn erosion_from(&self, p: DVec2, centre: &Wide, around: [f64; 4], octaves: usize) -> f64 {
        let [east, west, south, north] = around;
        // the slope of the big shape: rise over run, along x and along z
        let run = 2.0 * self.slope_step();
        let slope = DVec2::new((east - west) / run, (south - north) / run);
        // -1 in valleys, 1 on peaks: how far the hills pushed this spot, out of
        // about their full height
        let fade_target = centre.relief / (0.6 * self.0.erosion.fade_height);
        let eroded = erosion::erode(
            &self.0.erosion,
            self.0.erosion_seed,
            p,
            slope,
            fade_target,
            octaves,
        );
        // strongest in the mountains, faint on plains and hills (see
        // `ErosionSettings::follow_relief`), and fading out towards the coast
        let relief = if self.0.erosion.follow_relief { centre.mountains.min(1.0) } else { 1.0 };
        let fade = if self.0.erosion.inland { centre.land } else { 1.0 };
        eroded.height * relief * fade
    }

    /// The big shape and erosion at the `count` x `count` points `first + (i, j) *
    /// spacing` (metres), row by row. The big shape is sampled once per point on a
    /// grid reaching `slope_step` further out, and shared: every point's slope
    /// neighbours are points of the same grid (`spacing` must divide `slope_step`)
    /// Erosion is only computed where `wanted(i, j)`; elsewhere it's 0
    fn eroded_grid(
        &self,
        first: DVec2,
        spacing: f64,
        count: usize,
        octaves: usize,
        wanted: impl Fn(usize, usize) -> bool,
    ) -> Vec<(Wide, f64)> {
        if !self.0.erosion.enabled {
            return (0..count * count)
                .map(|i| {
                    let p = first + DVec2::new((i % count) as f64, (i / count) as f64) * spacing;
                    (self.wide_ground(p.x, p.y), 0.0)
                })
                .collect();
        }
        // grid points between a point and its slope neighbours
        let k = (self.slope_step() / spacing).round() as usize;
        let across = count + 2 * k;
        let start = first - DVec2::splat((k as f64) * spacing);
        let wide: Vec<Wide> = (0..across * across)
            .map(|i| {
                let p = start + DVec2::new((i % across) as f64, (i / across) as f64) * spacing;
                self.wide_ground(p.x, p.y)
            })
            .collect();
        let at = |i: usize, j: usize| &wide[i + j * across];
        (0..count * count)
            .map(|index| {
                let (i, j) = (index % count + k, index / count + k);
                let centre = *at(i, j);
                if !wanted(i - k, j - k) {
                    return (centre, 0.0);
                }
                let around = [
                    at(i + k, j).height,
                    at(i - k, j).height,
                    at(i, j + k).height,
                    at(i, j - k).height,
                ];
                let p = start + DVec2::new(i as f64, j as f64) * spacing;
                (centre, self.erosion_from(p, &centre, around, octaves))
            })
            .collect()
    }

    /// Erosion (metres) at one point, sampling what it needs itself
    fn erosion_point(&self, p: DVec2, octaves: usize) -> f64 {
        let step = self.slope_step();
        let centre = self.wide_ground(p.x, p.y);
        let around = [
            self.wide_ground(p.x + step, p.y).height,
            self.wide_ground(p.x - step, p.y).height,
            self.wide_ground(p.x, p.y + step).height,
            self.wide_ground(p.x, p.y - step).height,
        ];
        self.erosion_from(p, &centre, around, octaves)
    }

    /// Erosion at any point: a blend of the 4 `EROSION_GRID` points around it
    fn erosion_at(&self, x: f64, z: f64) -> f64 {
        if !self.0.erosion.enabled {
            return 0.0;
        }
        let corner = DVec2::new((x / EROSION_GRID).floor(), (z / EROSION_GRID).floor()) * EROSION_GRID;
        let octaves = self.0.erosion.octaves;
        let values = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
            .map(|(dx, dz)| self.erosion_point(corner + DVec2::new(dx, dz) * EROSION_GRID, octaves));
        bilinear(values, (x - corner.x) / EROSION_GRID, (z - corner.y) / EROSION_GRID)
    }

    /// The column of blocks at block `x`, `z`
    pub fn column_at(&self, x: i32, z: i32) -> Column {
        let neighbours = NEIGHBOURS.map(|(dx, dz)| self.raw_column(x + dx, z + dz));
        // the highest water standing within `shore_reach` blocks
        let reach = self.0.shore_reach;
        let nearby = (-reach..=reach)
            .flat_map(|dz| (-reach..=reach).map(move |dx| (dx, dz)))
            .filter_map(|(dx, dz)| self.raw_column(x + dx, z + dz).water_top())
            .max();
        self.finish_column(x, z, self.raw_column(x, z), neighbours, nearby)
    }

    /// Ground and water at block `x`, `z`, before the surface is picked. Sampled
    /// at the column's centre
    fn raw_column(&self, x: i32, z: i32) -> RawColumn {
        let (mx, mz) = column_centre(x, z);
        self.raw_column_with(x, z, self.erosion_at(mx, mz))
    }

    /// `raw_column`, with the erosion at the column's centre already worked out
    /// (`generate` blends it from a grid computed once per chunk)
    fn raw_column_with(&self, x: i32, z: i32, erosion: f64) -> RawColumn {
        // Noise works in metres, so settings mean the same thing whatever the
        // block size. Block 3 spans 1.5..2.0 m; its centre is (3 + 0.5) * 0.5 = 1.75 m
        let (x, z) = column_centre(x, z);

        // the big shape, carved by erosion, with the small detail on top
        let wide = self.wide_ground(x, z);
        let metres = wide.height + erosion + self.detail(x, z, wide.land);
        // back to whole blocks: 9.55 m / 0.5 = 19.1, rounds to 19. So blocks up to
        // 18 are ground, 18 is the top one, and 19 (9.5 m) is the first air
        let height = (metres / BLOCK_SIZE as f64).round() as i32;

        // the water surface here: its lake cell's level (see `Lakes::level_at`).
        // The ocean is part of the lake grid too, at sea level
        let level = self.0.lakes.level_at(x, z, self);
        // e.g. level 3.2 m / 0.5 = 6.4, rounds to 6: water in blocks below 6. On
        // ground above that (height >= 6), there's no water: `water` = `height`
        let water = ((level as f64 / BLOCK_SIZE as f64).round() as i32).max(height);

        RawColumn {
            metres,
            height,
            water,
        }
    }

    /// Picks a column's surface rule from its facts (see `surface.rs`).
    /// `neighbours` are the raw columns east, west, south and north of it, in the
    /// order of `NEIGHBOURS`. `nearby_water` is the top (in blocks) of the highest
    /// water standing within `shore_reach` blocks, if there is any
    fn finish_column(
        &self,
        x: i32,
        z: i32,
        raw: RawColumn,
        neighbours: [RawColumn; 4],
        nearby_water: Option<i32>,
    ) -> Column {
        let [east, west, south, north] = neighbours;
        // how much the ground rises across this column, between the neighbours on
        // either side (2 blocks apart). e.g. east 1 m higher than west: 45 degrees
        let slope = surface::slope_degrees(
            east.metres - west.metres,
            south.metres - north.metres,
            2.0 * BLOCK_SIZE as f64,
        );

        // metres: the surface of the water nearby, if any
        let nearby = nearby_water.map(|top| top as f64 * BLOCK_SIZE as f64);

        // the wobble shifts the altitude rules see: see `SurfaceSettings::wobble`
        let (mx, mz) = (
            (x as f64 + 0.5) * BLOCK_SIZE as f64,
            (z as f64 + 0.5) * BLOCK_SIZE as f64,
        );
        let wobble = self.0.wobble.get([mx, mz]) * self.0.wobble_height;

        let facts = Facts {
            altitude: raw.metres + wobble,
            slope,
            under_water: raw.water > raw.height,
            above_water: nearby.map(|surface| raw.metres - surface),
        };
        let (temperature, rainfall) = self.climate(mx, mz, raw.metres);
        let biome = self.biome(temperature, rainfall);
        Column {
            height: raw.height,
            water: raw.water,
            biome,
            surface: surface::pick(&self.0.biomes[biome].rules, &facts),
        }
    }

    /// (temperature in degrees Celsius, rainfall in cm/year) at (x, z) metres,
    /// where the ground is at `altitude` metres
    fn climate(&self, x: f64, z: f64, altitude: f64) -> (f64, f64) {
        let climate = &self.0.climate;
        // the noise is mostly within ±0.5: spread that over the range.
        // e.g. range -15..32 C, noise 0.1: 60% of the way, 13.2 C
        let spread = |noise: f64, (low, high): (f64, f64)| low + (high - low) * (noise + 0.5);
        let jitter = |offset: f64| self.0.jitter.get([x + offset, z]);

        // colder with height, like real air: e.g. 0.65 C per 100 m, at 40 m: -0.26 C
        let temperature = spread(self.0.temperature.get([x, z]), climate.temperature.range)
            - climate.lapse_rate * altitude.max(0.0) / 100.0
            + jitter(0.0) * climate.jitter.temperature;
        // the rainfall's jitter is sampled somewhere else, so the two don't move together
        let rainfall = spread(self.0.rainfall.get([x, z]), climate.rainfall.range)
            + jitter(10_000.0) * climate.jitter.rainfall;
        (temperature, rainfall.max(0.0))
    }

    /// The first biome whose ranges hold this climate, else the last one
    fn biome(&self, temperature: f64, rainfall: f64) -> usize {
        let within = |(min, max): (f64, f64), value: f64| min <= value && value < max;
        self.0
            .biomes
            .iter()
            .position(|biome| within(biome.temperature, temperature) && within(biome.rainfall, rainfall))
            .unwrap_or(self.0.biomes.len() - 1)
    }

    /// The point on top of the ground above `xz` (metres)
    pub fn surface_point(&self, xz: Vec2) -> Vec3 {
        // metres -> the block column containing that point
        let x = (xz.x / BLOCK_SIZE).floor() as i32;
        let z = (xz.y / BLOCK_SIZE).floor() as i32;
        // the ground height in blocks is also where the first air block starts,
        // so that height in metres is exactly the top of the ground
        Vec3::new(xz.x, self.raw_column(x, z).height as f32 * BLOCK_SIZE, xz.y)
    }

    /// The nearest point to `from` (metres) that is dry land, searching outward in
    /// rings 64 m apart. Used for spawn, which shouldn't be at the bottom of an
    /// ocean. Only if there's no land within 50 km does it give up and use `from`
    pub fn nearest_land(&self, from: Vec2) -> Vec2 {
        // dry: no water standing on the ground
        let on_land = |xz: Vec2| {
            let x = (xz.x / BLOCK_SIZE).floor() as i32;
            let z = (xz.y / BLOCK_SIZE).floor() as i32;
            self.raw_column(x, z).water_top().is_none()
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
        // and so on. Above the ground is negative
        let depth = column.height - 1 - y;
        if depth < 0 {
            // above the ground: water up to the column's water surface, then air.
            // e.g. height -6, water 0: blocks -6..=-1 are water, 0 and up air
            return if y < column.water {
                Block::WATER
            } else {
                Block::AIR
            };
        }
        // walk down the rule's layers until one reaches this depth. e.g. layers
        // grass 1, dirt 3: depth 0 grass, 1..=3 dirt, 4 and deeper stone
        let layers = self.0.biomes[column.biome]
            .rules
            .get(column.surface)
            .map_or(&[][..], |rule| &rule.layers[..]);
        let mut below = 0;
        for &(block, thick) in layers {
            below += thick;
            if depth < below {
                return block;
            }
        }
        Block::STONE
    }

    /// Step 1 of `generate`: the chunk's 32 x 32 columns, stored flat: column
    /// (x, z) is at x + z * 32
    fn columns(&self, pos: ChunkPos) -> Vec<Column> {
        // `pos` counts in chunks; `origin` is the world block at the chunk's
        // (0, 0, 0) corner. e.g. chunk (1, -1, 0) starts at block (32, -32, 0)
        let origin = pos.0 * CHUNK_SIZE;
        let size = CHUNK_SIZE as usize;

        // Step 1: each of the chunk's 32 x 32 columns. Columns only depend on x
        // and z, so each is sampled once here instead of once per block.
        // Picking a column's surface needs the columns around it (its 4 neighbours
        // for the slope, any water within `shore_reach` for beaches), so first
        // sample raw columns in a border `pad` wide all round, reaching into the
        // next chunks. Stored flat: raw (x, z) is at (x + pad) + (z + pad) * padded
        let pad = self.0.shore_reach.max(1);
        let padded = CHUNK_SIZE + 2 * pad;
        // erosion for all of them, on the `EROSION_GRID` points covering them
        // (plus one more row and column, for blending the last columns)
        let (min_x, min_z) = column_centre(origin.x - pad, origin.z - pad);
        let (max_x, _) = column_centre(origin.x + CHUNK_SIZE + pad - 1, 0);
        let grid_first = DVec2::new((min_x / EROSION_GRID).floor(), (min_z / EROSION_GRID).floor())
            * EROSION_GRID;
        let grid_count = ((max_x - grid_first.x) / EROSION_GRID).floor() as usize + 2;
        let erosion: Vec<f64> = if self.0.erosion.enabled {
            self.eroded_grid(grid_first, EROSION_GRID, grid_count, self.0.erosion.octaves, |_, _| true)
                .into_iter()
                .map(|(_, erosion)| erosion)
                .collect()
        } else {
            Vec::new()
        };
        let erosion_at = |x: f64, z: f64| {
            if erosion.is_empty() {
                return 0.0;
            }
            let gx = ((x - grid_first.x) / EROSION_GRID).floor() as usize;
            let gz = ((z - grid_first.y) / EROSION_GRID).floor() as usize;
            let value = |i: usize, j: usize| erosion[(gx + i) + (gz + j) * grid_count];
            bilinear(
                [value(0, 0), value(1, 0), value(0, 1), value(1, 1)],
                (x - grid_first.x) / EROSION_GRID - gx as f64,
                (z - grid_first.y) / EROSION_GRID - gz as f64,
            )
        };
        let mut raw = Vec::with_capacity((padded * padded) as usize);
        for z in -pad..CHUNK_SIZE + pad {
            for x in -pad..CHUNK_SIZE + pad {
                let (block_x, block_z) = (origin.x + x, origin.z + z);
                let (mx, mz) = column_centre(block_x, block_z);
                raw.push(self.raw_column_with(block_x, block_z, erosion_at(mx, mz)));
            }
        }
        let raw_at = |x: i32, z: i32| raw[((x + pad) + (z + pad) * padded) as usize];
        let reach = self.0.shore_reach;
        // then the finished columns, stored flat: column (x, z) is at x + z * 32
        let mut columns = Vec::with_capacity(size * size);
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let neighbours = NEIGHBOURS.map(|(dx, dz)| raw_at(x + dx, z + dz));
                // the highest water standing within `shore_reach` blocks
                let nearby = (-reach..=reach)
                    .flat_map(|dz| (-reach..=reach).map(move |dx| (dx, dz)))
                    .filter_map(|(dx, dz)| raw_at(x + dx, z + dz).water_top())
                    .max();
                columns.push(self.finish_column(
                    origin.x + x,
                    origin.z + z,
                    raw_at(x, z),
                    neighbours,
                    nearby,
                ));
            }
        }

        columns
    }

    /// All the blocks of one chunk (the tests' shorthand)
    #[cfg(test)]
    pub fn generate(&self, pos: ChunkPos) -> Chunk {
        self.generate_with_sky(pos).0
    }

    /// All the blocks of one chunk, and its columns' heights: the first block
    /// up with only air above (the top of the ground, or of water on it; see
    /// `world/sky.rs`), x + z * 32. Runs on a background thread for every chunk
    /// the server needs, so it should stay fast
    pub fn generate_with_sky(&self, pos: ChunkPos) -> (Chunk, Vec<i32>) {
        let columns = self.columns(pos);
        let heights = columns.iter().map(|column| column.water).collect();
        (self.blocks(pos, &columns), heights)
    }

    /// Every block of the chunk at `pos`, from its columns
    fn blocks(&self, pos: ChunkPos, columns: &[Column]) -> Chunk {
        let origin = pos.0 * CHUNK_SIZE;

        // Shortcut: most chunks are entirely sky, water or underground. Compare
        // the chunk's vertical span (world block heights `bottom..=top`) with the
        // lowest and highest ground in it, and the water surfaces:
        //
        //   bottom >= every      nothing but air: the chunk starts above all
        //   ground and water     ground and all water
        //   bottom >= highest    no ground in the chunk, and it ends below every
        //   top < lowest water   column's water surface: all water
        //   top below the        every block is deeper than even the thickest
        //   lowest layers        surface layers under the lowest ground: all stone
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
        // the lowest column's top block is at `lowest - 1`, and no rule's layers go
        // more than `deepest_layers` down from a top block
        if top < lowest - self.0.deepest_layers {
            return Chunk::Uniform(Block::STONE);
        }

        // Step 2: the chunk crosses the ground or the sea's surface somewhere
        self.fill(origin.y, columns)
    }

    /// Step 2 of `generate`: every block of the chunk, column by column, for a
    /// chunk whose lowest layer is at world height `bottom`. `Chunk::default()` is
    /// all air, so air blocks need no work
    fn fill(&self, bottom: i32, columns: &[Column]) -> Chunk {
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

/// The lake grid's view of the terrain: the big shape with the coarse erosion
/// octaves, lowered by the most the small detail and the fine erosion octaves
/// (left out: too small for its grid) could lower the ground
impl SampleGrid for Terrain {
    fn sample_grid(&self, first: DVec2, spacing: f64, count: usize) -> Vec<CellSample> {
        let too_narrow = self.too_narrow();
        let detail_dip: f64 = self
            .0
            .layers
            .iter()
            .filter(|layer| layer.scale < too_narrow)
            .map(|layer| layer.height)
            .sum();
        let erosion_dip = if self.0.erosion.enabled {
            octave_bound(&self.0.erosion, self.0.coarse_octaves)
        } else {
            0.0
        };
        self.eroded_grid(first, spacing, count, self.0.coarse_octaves, |i, j| (i + j) % 2 == 0)
            .into_iter()
            .map(|(wide, erosion)| CellSample {
                height: (wide.height + erosion - detail_dip - erosion_dip) as f32,
                ocean: wide.continentalness < self.0.coast,
            })
            .collect()
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
        // `generate` (with its whole-chunk shortcuts) against filling every block
        let terrain = Terrain::new(&WorldGenSettings::default());
        for pos in sample_chunks() {
            let chunk = terrain.generate(pos);
            let filled = terrain.fill(pos.0.y * CHUNK_SIZE, &terrain.columns(pos));
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        let local = IVec3::new(x, y, z);
                        assert_eq!(chunk.get(local), filled.get(local), "{:?} {local}", pos.0);
                    }
                }
            }
        }
    }

    #[test]
    fn columns_are_grass_then_dirt_then_stone() {
        // in the default temperate forest (no rules of its own): rule 0 is the global
        // under water rule (sand 4), and the last is its ground, grass 1, dirt 3
        let settings = WorldGenSettings::default();
        let terrain = Terrain::new(&settings);
        let forest = biome_named(&settings, "temperate forest");
        let land = Column {
            height: 10,
            water: 10,
            biome: forest,
            surface: terrain.0.biomes[forest].rules.len() - 1,
        };
        assert_eq!(terrain.block(land, 10), Block::AIR);
        assert_eq!(terrain.block(land, 9), Block::GRASS);
        assert_eq!(terrain.block(land, 6), Block::DIRT);
        assert_eq!(terrain.block(land, 5), Block::STONE);

        let lake = Column {
            water: 12,
            surface: 0,
            ..land
        };
        assert_eq!(terrain.block(lake, 11), Block::WATER);
        assert_eq!(terrain.block(lake, 9), Block::SAND);
        assert_eq!(terrain.block(lake, 6), Block::SAND);
        assert_eq!(terrain.block(lake, 5), Block::STONE);

        // past the end of the rules: bare stone right to the top
        let bare = Column {
            surface: terrain.0.biomes[forest].rules.len(),
            ..land
        };
        assert_eq!(terrain.block(bare, 9), Block::STONE);
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
        let x = (land.x / BLOCK_SIZE).floor() as i32;
        let z = (land.y / BLOCK_SIZE).floor() as i32;
        assert!(terrain.raw_column(x, z).water_top().is_none());
    }

    #[test]
    fn oceans_fill_to_sea_level() {
        let settings = WorldGenSettings::default();
        let terrain = Terrain::new(&settings);
        // rule 0 in the temperate forest: the global under water rule, 4 sand
        let deep = Column {
            height: -10,
            water: 0,
            biome: biome_named(&settings, "temperate forest"),
            surface: 0,
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
                    let here = terrain.raw_column(x, z);
                    for there in [terrain.raw_column(x + 1, z), terrain.raw_column(x, z + 1)] {
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

    fn biome_named(settings: &WorldGenSettings, name: &str) -> usize {
        settings.biomes.iter().position(|b| b.name == name).unwrap()
    }

    #[test]
    fn higher_is_colder() {
        let terrain = Terrain::new(&WorldGenSettings::default());
        let (low, _) = terrain.climate(100.0, 200.0, 0.0);
        let (high, _) = terrain.climate(100.0, 200.0, 1000.0);
        assert!((low - high - 6.5).abs() < 1e-9, "{low} vs {high}");
    }

    #[test]
    fn the_first_biome_whose_ranges_hold_wins() {
        let settings = WorldGenSettings::default();
        let terrain = Terrain::new(&settings);
        let named = |name| biome_named(&settings, name);
        assert_eq!(terrain.biome(-5.0, 300.0), named("tundra"));
        assert_eq!(terrain.biome(12.0, 50.0), named("grassland"));
        assert_eq!(terrain.biome(12.0, 250.0), named("temperate forest"));
        assert_eq!(terrain.biome(28.0, 20.0), named("desert"));
        assert_eq!(terrain.biome(28.0, 300.0), named("tropical forest"));
    }

    #[test]
    fn biome_rules_come_before_global_ones() {
        // a steep slope: gravel scree by the global rules, but sand in a desert
        let settings = WorldGenSettings::default();
        let terrain = Terrain::new(&settings);
        let steep = Facts {
            altitude: 10.0,
            slope: 40.0,
            under_water: false,
            above_water: None,
        };
        let first_layer = |biome: usize| {
            let rules = &terrain.0.biomes[biome].rules;
            rules[surface::pick(rules, &steep)].layers[0].0
        };
        assert_eq!(first_layer(biome_named(&settings, "desert")), Block::SAND);
        assert_eq!(first_layer(biome_named(&settings, "temperate forest")), Block::GRAVEL);

        // flat ground: no rule matches, so each biome's ground
        let flat = Facts { slope: 0.0, ..steep };
        let ground = |biome: usize| {
            let rules = &terrain.0.biomes[biome].rules;
            rules[surface::pick(rules, &flat)].layers[0].0
        };
        assert_eq!(ground(biome_named(&settings, "taiga")), Block::PODZOL);
        assert_eq!(ground(biome_named(&settings, "savanna")), Block::DRY_GRASS);
    }
}
