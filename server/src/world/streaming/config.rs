use bevy::prelude::*;

/// The farthest players can see, in chunks from the one they stand in (x, y, z).
/// Their render distance setting can bring x and z closer. One chunk is 16 m. The outermost ring is loaded but not drawn, since drawing a
/// chunk needs all its neighbours
pub const VIEW_DISTANCE: IVec3 = IVec3::new(8, 4, 8);

/// Chunks past the view distance a client keeps before being told to drop them,
/// so walking back and forth over a border doesn't resend the same chunks
pub const UNLOAD_MARGIN: i32 = 2;

/// Rough bytes of chunk data sent to each client per tick (64 ticks a second),
/// 4 KiB = 256 KiB/s. Uniform chunks are ~20 bytes, mixed ones 8-16 KiB. Sending
/// faster than the link carries only builds a queue (and resends); keep it
/// under what a player's connection can really take
pub const SEND_BYTES_PER_TICK: usize = 4 * 1024;
