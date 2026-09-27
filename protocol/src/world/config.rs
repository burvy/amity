/// Blocks along each edge of a chunk. A chunk is CHUNK_SIZE^3 blocks
pub const CHUNK_SIZE: i32 = 32;
/// Edge length of one block in metres. Everything else in the voxel code counts in
/// blocks; convert to metres only through `block_corner` and `block_at`.
pub const BLOCK_SIZE: f32 = 0.5;

/// How far a player can break and place blocks
pub const REACH: f32 = 5.0;
/// Extra distance the server allows when checking an edit: the aimed point can be
/// up to ~0.87 from the block's center, plus the player may have moved since
pub const REACH_TOLERANCE: f32 = 0.5;
