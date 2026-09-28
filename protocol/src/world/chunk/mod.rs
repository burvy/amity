//! Chunks: CHUNK_SIZE^3 cubes of blocks, one entity each

use avian3d::prelude::*;
use bevy::prelude::*;

pub mod collider;
pub mod pos;
pub mod sky;
pub mod storage;

pub use pos::{ChunkIndex, ChunkPos};
pub use sky::{ChunkSky, ColumnSky};
pub use storage::Chunk;

/// Agreed-upon chunk entity for both client and server
/// The collider is added by `collider::init_colliders`, since an all-air chunk has none.
/// Transform is explicit because nothing else would add it to a chunk without a collider.
pub fn chunk_body(pos: ChunkPos, chunk: Chunk) -> impl Bundle {
    (
        pos,
        Position(pos.origin()),
        Transform::from_translation(pos.origin()),
        RigidBody::Static,
        chunk,
    )
}
