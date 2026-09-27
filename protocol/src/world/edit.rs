use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    aim::{self, Aim},
    protocol::PlayerInputs,
    world::chunk::{Block, Chunk, ChunkIndex, ChunkPos},
};

/// How far a player can break and place blocks
pub const REACH: f32 = 5.0;
/// What gets placed until there is an inventory
pub const PLACED_BLOCK: Block = Block::Stone;

/// Server -> client: the authoritative new value of one block
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockChanged {
    pub pos: IVec3,
    pub block: Block,
}

/// Reads and writes blocks by world position
#[derive(SystemParam)]
pub struct Blocks<'w, 's> {
    index: Res<'w, ChunkIndex>,
    chunks: Query<'w, 's, &'static mut Chunk>,
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
        let Some(mut chunk) = self
            .index
            .get(chunk)
            .and_then(|entity| self.chunks.get_mut(entity).ok())
        else {
            return false;
        };
        // if the block didn't change
        if chunk.get(local) == block {
            return false;
        }
        chunk.set(local, block);
        true
    }

    pub fn is_chunk(&self, entity: Entity) -> bool {
        self.chunks.contains(entity)
    }
}

/// The blocks on either side of an aimed-at chunk face
pub struct EditTarget {
    /// the solid block being looked at
    pub hit: IVec3,
    /// the block in front of the hit face, where placing goes
    pub adjacent: IVec3,
}

impl EditTarget {
    /// the hit point sits exactly on a face; half a block in or out lands firmly
    /// inside the hit block or the one in front of it
    pub fn from_aim(aim: &Aim) -> Self {
        Self {
            hit: (aim.point - aim.normal * 0.5).floor().as_ivec3(),
            adjacent: (aim.point + aim.normal * 0.5).floor().as_ivec3(),
        }
    }
}

/// The block change a player's held input asks for this tick, if any. Break wins
/// when both are held. Shared so the client predicts exactly what the server does.
pub fn player_edit(
    input: &PlayerInputs,
    player: Entity,
    position: Vec3,
    spatial: &SpatialQuery,
    blocks: &Blocks,
) -> Option<(IVec3, Block)> {
    if !input.break_block && !input.place_block {
        return None;
    }
    let aim = aim::aim(spatial, player, position, input.look, REACH)?;
    // anything else in the way (a player, a prop) blocks the edit
    if !blocks.is_chunk(aim.entity) {
        return None;
    }
    let target = EditTarget::from_aim(&aim);
    if input.break_block {
        return Some((target.hit, Block::Air));
    }

    // anything but a chunk (players, props) blocks placing into its space. Slightly
    // shrunk so something merely touching the block's faces doesn't count
    let occupied = spatial
        .shape_intersections(
            &Collider::cuboid(0.99, 0.99, 0.99),
            target.adjacent.as_vec3() + Vec3::splat(0.5),
            Quat::IDENTITY,
            &SpatialQueryFilter::default(),
        )
        .into_iter()
        .any(|entity| !blocks.is_chunk(entity));
    let free = blocks.get(target.adjacent) == Some(Block::Air) && !occupied;
    free.then_some((target.adjacent, PLACED_BLOCK))
}

/// Keeps each chunk's collider matching its blocks, including new chunks. A chunk
/// that becomes all air loses its collider.
pub fn sync_colliders(mut cmds: Commands, chunks: Query<(Entity, &Chunk), Changed<Chunk>>) {
    for (entity, chunk) in &chunks {
        match chunk.collider() {
            Some(collider) => cmds.entity(entity).insert(collider),
            None => cmds.entity(entity).remove::<Collider>(),
        };
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::SystemState;

    use super::*;
    use crate::world::chunk::{self, CHUNK_SIZE};

    #[test]
    fn negative_blocks_land_in_negative_chunks() {
        assert_eq!(
            ChunkPos::of_block(IVec3::new(-1, 0, 31)),
            (ChunkPos(IVec3::new(-1, 0, 0)), IVec3::new(31, 0, 31))
        );
        assert_eq!(
            ChunkPos::of_block(IVec3::splat(-CHUNK_SIZE)),
            (ChunkPos(IVec3::NEG_ONE), IVec3::ZERO)
        );
    }

    #[test]
    fn set_reports_only_real_changes() {
        let mut world = World::new();
        world.init_resource::<ChunkIndex>();
        world.add_observer(chunk::index_chunk);
        world.spawn((ChunkPos(IVec3::NEG_ONE), Chunk::default()));

        let mut state = SystemState::<Blocks>::new(&mut world);
        let mut blocks = state.get_mut(&mut world).unwrap();
        let pos = IVec3::new(-1, -5, -32);
        assert_eq!(blocks.get(pos), Some(Block::Air));
        assert!(blocks.set(pos, Block::Stone));
        assert!(!blocks.set(pos, Block::Stone));
        assert_eq!(blocks.get(pos), Some(Block::Stone));
        assert_eq!(blocks.get(IVec3::ZERO), None);
    }
}
