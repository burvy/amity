use bevy::prelude::*;

/// Chunks generated beyond what players can see (x, y, z, in chunks). Bigger
/// means players can move faster without catching up to generation, but more
/// chunks sit in memory
pub const PREGEN_MARGIN: IVec3 = IVec3::new(4, 2, 4);

/// Chunks generated at the same time. Higher fills the world faster, but they
/// share the background threads with everything else (a region's lake flooding
/// can hold one for a second), and too many at once starves the game loop
pub const MAX_GENERATING: usize = 8;

/// Structures (trees...) are placed at most one per square cell this many
/// blocks across (see `structures.rs`). Smaller allows denser forests, but
/// each chunk checks more spots
pub const STRUCTURE_CELL: i32 = 8;
