//! Everything the voxel world sends over the network, and the channels it uses

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    block::Block,
    chunk::{Chunk, ChunkPos},
};

/// Server -> client, ordered + reliable. Carries chunks and block changes, so a
/// block change can never overtake the chunk it modifies
pub struct WorldChannel;

/// Client -> server, ordered + reliable. Edits get their own channel so they are
/// never stuck behind a large chunk transfer
pub struct EditChannel;

/// Server -> client: the full contents of one chunk. Sending it again replaces
/// the client's copy
#[derive(Serialize, Deserialize, Clone)]
pub struct ChunkData {
    pub pos: ChunkPos,
    pub chunk: Chunk,
}

/// Server -> client: this chunk is out of range, drop it
#[derive(Serialize, Deserialize, Clone)]
pub struct UnloadChunk {
    pub pos: ChunkPos,
}

/// Server -> client: the authoritative new value of one block
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockChanged {
    pub pos: IVec3,
    pub block: Block,
}

/// Client -> server: "set this block", chosen by the client's own aim. Sent
/// reliably, so unlike per-tick inputs it is never guessed, dropped or skipped
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EditBlock {
    pub pos: IVec3,
    pub block: Block,
}
