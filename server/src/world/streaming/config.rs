use bevy::prelude::*;

/// How far players see, in chunks from the one they stand in (x, y, z). One
/// chunk is 16 m. The outermost ring is loaded but not drawn, since drawing a
/// chunk needs all its neighbours
pub const VIEW_DISTANCE: IVec3 = IVec3::new(8, 4, 8);

/// Chunks past the view distance a client keeps before being told to drop them,
/// so walking back and forth over a border doesn't resend the same chunks
pub const UNLOAD_MARGIN: i32 = 2;

/// Rough bytes of chunk data sent to each client per tick (64 ticks a second).
/// Uniform chunks are ~20 bytes, mixed ones ~8 KiB
pub const SEND_BYTES_PER_TICK: usize = 16 * 1024;
