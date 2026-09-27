//! Reading and writing single blocks by world position

use bevy::{ecs::system::SystemParam, prelude::*};

use super::{
    block::Block,
    chunk::{Chunk, ChunkIndex, ChunkPos},
};

/// One block that changed after its chunk was spawned. Colliders and meshes
/// update from these instead of rebuilding a whole chunk per edit
#[derive(Message, Clone, Copy, Debug)]
pub struct BlockSet {
    pub chunk: Entity,
    pub local: IVec3,
    pub block: Block,
}

/// Reads and writes blocks by world position
#[derive(SystemParam)]
pub struct Blocks<'w, 's> {
    index: Res<'w, ChunkIndex>,
    chunks: Query<'w, 's, &'static mut Chunk>,
    changes: MessageWriter<'w, BlockSet>,
}

impl Blocks<'_, '_> {
    /// None if the chunk holding `pos` isn't loaded
    pub fn get(&self, pos: IVec3) -> Option<Block> {
        let (chunk, local) = ChunkPos::of_block(pos);
        let chunk = self.chunks.get(self.index.get(chunk)?).ok()?;
        Some(chunk.get(local))
    }

    /// Returns true if the block changed. Setting a block to what it already is
    /// leaves the chunk untouched, so it doesn't trigger a remesh or collider rebuild.
    pub fn set(&mut self, pos: IVec3, block: Block) -> bool {
        let (chunk, local) = ChunkPos::of_block(pos);
        // if the chunk exists
        let Some(entity) = self.index.get(chunk) else {
            return false;
        };
        let Ok(mut chunk) = self.chunks.get_mut(entity) else {
            return false;
        };
        // if the block didn't change
        if chunk.get(local) == block {
            return false;
        }
        chunk.set(local, block);
        self.changes.write(BlockSet {
            chunk: entity,
            local,
            block,
        });
        true
    }

    pub fn is_chunk(&self, entity: Entity) -> bool {
        self.chunks.contains(entity)
    }
}
