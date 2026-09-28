//! Horizon shadows: sizes and softness

/// Blocks per side of a horizon cell. The map keeps one horizon per cell, so
/// this is how fine far shadows are: 4 blocks is 2 m, plenty for mountains and
/// hills (the shadow map handles small things near you)
pub const CELL_BLOCKS: i32 = 4;

/// Cells per side of the map. It follows the player, so this covers the
/// loaded area: 128 cells of 2 m is 256 m, the widest render distance (8
/// chunks of 16 m each way)
pub const MAP_CELLS: i32 = 128;

/// How far each cell looks for its horizon, in cells (64 is 128 m). Further
/// finds the shadows of more distant peaks, at more cost
pub const SEARCH_CELLS: i32 = 64;

/// Metres a cell's own ground is raised by when looking for its horizon, so
/// gentle slopes and bumps within a cell don't shadow themselves
pub const SELF_SHADOW_BIAS: f32 = 1.0;

/// Degrees over which light fades out as it sets behind the horizon: the sun
/// is 0.5 degrees across (its real shadows' soft edge), and the horizon is
/// only known per cell. Larger blurs far shadows more
pub const SOFTNESS_DEG: f32 = 0.75;

/// The shadow map stops at its reach (the Shadows setting). Over this last
/// share of it, its sharp shadows fade into the horizon's, so there's no edge
/// where one ends and the other begins
pub const SHADOW_FADE: f32 = 0.3;

/// Seconds between redoing the map while chunks keep arriving or changing
pub const UPDATE_SECS: f32 = 1.0;

/// Cells the player can move from where the map was last centred before it's
/// redone around them
pub const RECENTRE_CELLS: i32 = 8;
