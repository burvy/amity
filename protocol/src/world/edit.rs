use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    aim::{self, Aim},
    world::chunk::{block_at, block_corner, Block, Chunk, ChunkIndex, ChunkPos, BLOCK_SIZE},
};

/// How far a player can break and place blocks
pub const REACH: f32 = 5.0;
/// Extra distance the server allows when checking an edit: the aimed point can be
/// up to ~0.87 from the block's center, plus the player may have moved since
pub const REACH_TOLERANCE: f32 = 0.5;
/// What gets placed until there is an inventory
pub const PLACED_BLOCK: Block = Block::Stone;

/// Client -> server: "set this block", chosen by the client's own aim. Sent
/// reliably, so unlike per-tick inputs it is never guessed, dropped or skipped
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EditBlock {
    pub pos: IVec3,
    pub block: Block,
}

/// Ordered + reliable, client -> server
pub struct EditChannel;

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
            hit: block_at(aim.point - aim.normal * BLOCK_SIZE / 2.0),
            adjacent: block_at(aim.point + aim.normal * BLOCK_SIZE / 2.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditAction {
    Break,
    Place,
}

/// The block change an action aimed along `look` would make, if any
pub fn player_edit(
    action: EditAction,
    player: Entity,
    position: Vec3,
    look: Vec2,
    spatial: &SpatialQuery,
    blocks: &Blocks,
) -> Option<(IVec3, Block)> {
    let aim = aim::aim(spatial, player, position, look, REACH)?;
    // anything else in the way (a player, a prop) blocks the edit
    if !blocks.is_chunk(aim.entity) {
        return None;
    }
    let target = EditTarget::from_aim(&aim);
    match action {
        EditAction::Break => Some((target.hit, Block::Air)),
        EditAction::Place => {
            can_place(target.adjacent, spatial, blocks).then_some((target.adjacent, PLACED_BLOCK))
        }
    }
}

/// A block can go at `pos` if it is air and nothing but chunks overlaps it
/// (players, props). Slightly shrunk so something merely touching the block's
/// faces doesn't count
pub fn can_place(pos: IVec3, spatial: &SpatialQuery, blocks: &Blocks) -> bool {
    let occupied = spatial
        .shape_intersections(
            &Collider::cuboid(
                BLOCK_SIZE * 0.99,
                BLOCK_SIZE * 0.99,
                BLOCK_SIZE * 0.99,
            ),
            block_corner(pos) + Vec3::splat(BLOCK_SIZE / 2.0),
            Quat::IDENTITY,
            &SpatialQueryFilter::default(),
        )
        .into_iter()
        .any(|entity| !blocks.is_chunk(entity));
    blocks.get(pos) == Some(Block::Air) && !occupied
}

/// The server's check of a client's `EditBlock`: within reach of where the server
/// has the player, and a legal change of that block right now
pub fn edit_allowed(edit: &EditBlock, eye: Vec3, spatial: &SpatialQuery, blocks: &Blocks) -> bool {
    let center = block_corner(edit.pos) + Vec3::splat(BLOCK_SIZE / 2.0);
    if eye.distance(center) > REACH + REACH_TOLERANCE {
        return false;
    }
    match edit.block {
        Block::Air => blocks.get(edit.pos).is_some_and(Block::is_solid),
        block => block == PLACED_BLOCK && can_place(edit.pos, spatial, blocks),
    }
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
    fn metres_convert_to_blocks() {
        use crate::world::chunk::{block_at, block_corner, BLOCK_SIZE};
        assert_eq!(block_at(Vec3::new(-0.01, 0.0, BLOCK_SIZE - 0.01)), IVec3::new(-1, 0, 0));
        assert_eq!(block_at(Vec3::splat(BLOCK_SIZE)), IVec3::ONE);
        assert_eq!(block_at(block_corner(IVec3::new(-3, 7, 0))), IVec3::new(-3, 7, 0));
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
