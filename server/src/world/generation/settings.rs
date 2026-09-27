//! What the generated world looks like: the numbers to tweak. `terrain.rs`
//! explains how they are used.
//!
//! Units: heights and sizes are in metres (a block is BLOCK_SIZE = 0.5 m),
//! depths counted in blocks say so. y = 0 m is sea level.

use bevy::prelude::*;

/// One world's generation settings. Later this can be loaded from a preset file,
/// one per world type
#[derive(Resource, Clone, Debug)]
pub struct WorldGenSettings {
    /// Picks which world you get. Same seed + same settings = the same world,
    /// block for block, every time
    pub seed: u32,
    /// Ground height (metres) where every layer happens to sample 0. The
    /// average height of the world
    pub base_height: f64,
    /// Blocks of dirt between the grass and the stone
    pub dirt_depth: i32,
    /// Noise layers added together to make the ground height. See `NoiseLayer`
    pub layers: Vec<NoiseLayer>,
}

impl Default for WorldGenSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            base_height: 0.0,
            dirt_depth: 3,
            layers: vec![
                // the rolling shape of the land
                NoiseLayer {
                    scale: 250.0,
                    height: 50.0,
                    octaves: 5,
                    persistence: 0.6,
                    lacunarity: 2.0,
                },
                // small lumps you can walk over
                NoiseLayer {
                    scale: 30.0,
                    height: 1.5,
                    octaves: 2,
                    persistence: 0.5,
                    lacunarity: 2.0,
                },
            ],
        }
    }
}

/// One layer of fractal noise (fBm) that raises and lowers the ground.
///
/// fBm adds up several copies ("octaves") of smooth Perlin noise. The first is
/// `scale` wide and moves the ground by up to about `height`; each next octave is
/// `lacunarity` times smaller and `persistence` times weaker. Big shapes with
/// smaller bumps on top, like real land.
#[derive(Clone, Debug)]
pub struct NoiseLayer {
    /// Metres across the biggest features (roughly hill to hill). Raise for wider,
    /// gentler land; lower for tighter, busier land
    pub scale: f64,
    /// Metres the layer moves the ground up or down, at most (most places move
    /// much less). Raise for taller hills and deeper valleys
    pub height: f64,
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
