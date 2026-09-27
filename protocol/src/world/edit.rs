//! The rules for breaking and placing blocks, shared so the client asks for
//! exactly what the server will check

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{
    block::{block_at, block_corner, Block},
    blocks::Blocks,
    config::{BLOCK_SIZE, PLACED_BLOCK, REACH, REACH_TOLERANCE},
    messages::EditBlock,
};
use crate::player::aim::{self, Aim};

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
        EditAction::Break => Some((target.hit, Block::AIR)),
        EditAction::Place => {
            can_place(target.adjacent, spatial, blocks).then_some((target.adjacent, PLACED_BLOCK))
        }
    }
}

/// A block can go at `pos` if it is air or a fluid (placing into water replaces
/// it) and nothing but chunks overlaps it
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
    let replaceable = blocks
        .get(pos)
        .is_some_and(|block| block == Block::AIR || block.is_fluid());
    replaceable && !occupied
}

/// The server's check of a client's `EditBlock`: within reach of where the server
/// has the player, and a legal change of that block right now
pub fn edit_allowed(edit: &EditBlock, eye: Vec3, spatial: &SpatialQuery, blocks: &Blocks) -> bool {
    let center = block_corner(edit.pos) + Vec3::splat(BLOCK_SIZE / 2.0);
    if eye.distance(center) > REACH + REACH_TOLERANCE {
        return false;
    }
    match edit.block {
        Block::AIR => blocks.get(edit.pos).is_some_and(Block::is_solid),
        block => block == PLACED_BLOCK && can_place(edit.pos, spatial, blocks),
    }
}
