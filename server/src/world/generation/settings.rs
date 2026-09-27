use bevy::prelude::*;

/// What the generated world looks like. One of these per world; later it can be
/// loaded from a preset file, and a seed goes here once there is noise to seed.
/// Heights are in blocks (BLOCK_SIZE metres each), y = 0 is sea level.
#[derive(Resource, Clone, Debug)]
pub struct WorldGenSettings {
    /// The height of the top of the ground: grass is the block just below it
    pub ground_level: i32,
    /// Blocks of dirt between the grass and the stone
    pub dirt_depth: i32,
}

impl Default for WorldGenSettings {
    fn default() -> Self {
        Self {
            ground_level: 0,
            dirt_depth: 3,
        }
    }
}
