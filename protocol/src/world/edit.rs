//! The rules for breaking and placing blocks, shared so the client asks for
//! exactly what the server will check

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{
    block::{block_at, block_corner, Block},
    blocks::Blocks,
    config::{BLOCK_SIZE, BREAK_SIZE, REACH, REACH_TOLERANCE},
    messages::EditBlock,
};
use crate::player::aim::{self, Aim};

/// The blocks on either side of an aimed-at chunk face
pub struct EditTarget {
    /// the solid block being looked at
    pub hit: IVec3,
    /// the block in front of the hit face, where placing goes
    pub adjacent: IVec3,
    /// the lowest corner of the cube breaking clears (see `break_corner`)
    pub break_corner: IVec3,
}

impl EditTarget {
    /// the hit point sits exactly on a face; half a block in or out lands firmly
    /// inside the hit block or the one in front of it
    pub fn from_aim(aim: &Aim) -> Self {
        let hit = block_at(aim.point - aim.normal * BLOCK_SIZE / 2.0);
        Self {
            hit,
            adjacent: block_at(aim.point + aim.normal * BLOCK_SIZE / 2.0),
            break_corner: break_corner(hit, aim.point, aim.normal),
        }
    }
}

/// Where the cube that breaking clears starts, so it sits as close to the aimed
/// point as it can while including the hit block. Along the hit face's normal
/// it goes into the surface, away from the player. Along the face's other two
/// axes it goes toward whichever half of the block face was hit: aim at the
/// upper left of a wall block and the cube takes it and the blocks up and left
fn break_corner(hit: IVec3, point: Vec3, normal: Vec3) -> IVec3 {
    let reach = BREAK_SIZE - 1;
    // where in the hit block the aim landed, 0 to 1 along each axis
    let within = (point - block_corner(hit)) / BLOCK_SIZE;
    let facing = normal.abs().max_element();
    IVec3::from_array(std::array::from_fn(|axis| {
        let toward_high = if normal[axis].abs() == facing {
            // into the surface: the face looks out toward +axis, so go -axis
            normal[axis] < 0.0
        } else {
            within[axis] >= 0.5
        };
        if toward_high {
            hit[axis]
        } else {
            hit[axis] - reach
        }
    }))
}

/// The blocks of the break cube starting at `corner`
pub fn break_cube(corner: IVec3) -> impl Iterator<Item = IVec3> {
    (0..BREAK_SIZE).flat_map(move |y| {
        (0..BREAK_SIZE)
            .flat_map(move |z| (0..BREAK_SIZE).map(move |x| corner + IVec3::new(x, y, z)))
    })
}

/// The centre of the break cube starting at `corner`, in metres
fn break_cube_center(corner: IVec3) -> Vec3 {
    block_corner(corner) + Vec3::splat(BREAK_SIZE as f32 * BLOCK_SIZE / 2.0)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditAction {
    Break,
    Place,
}

/// The edit an action aimed along `look` would make, if any. `place` is the
/// block the player's selected item places, if it places one
pub fn player_edit(
    action: EditAction,
    place: Option<Block>,
    player: Entity,
    position: Vec3,
    look: Vec2,
    spatial: &SpatialQuery,
    blocks: &Blocks,
) -> Option<EditBlock> {
    let aim = aim::aim(spatial, player, position, look, REACH)?;
    // anything else in the way (a player, a prop) blocks the edit
    if !blocks.is_chunk(aim.entity) {
        return None;
    }
    let target = EditTarget::from_aim(&aim);
    match action {
        EditAction::Break => Some(EditBlock::Break {
            corner: target.break_corner,
        }),
        EditAction::Place => {
            let block = place?;
            can_place(target.adjacent, spatial, blocks).then_some(EditBlock::Place {
                pos: target.adjacent,
                block,
            })
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
/// has the player, and a legal edit right now (a break needs something solid in
/// its cube). Whether the player has the block to place is the inventory's
/// check, done separately
pub fn edit_allowed(edit: &EditBlock, eye: Vec3, spatial: &SpatialQuery, blocks: &Blocks) -> bool {
    match *edit {
        EditBlock::Break { corner } => {
            // the hit block is within reach; the cube's centre is at most one
            // block further than it
            eye.distance(break_cube_center(corner)) <= REACH + REACH_TOLERANCE + BLOCK_SIZE
                && break_cube(corner).any(|pos| blocks.get(pos).is_some_and(Block::is_solid))
        }
        EditBlock::Place { pos, .. } => {
            let center = block_corner(pos) + Vec3::splat(BLOCK_SIZE / 2.0);
            eye.distance(center) <= REACH + REACH_TOLERANCE && can_place(pos, spatial, blocks)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A point on the given face of block `hit`, at `within` (0 to 1 along
    /// each axis) inside it
    fn aimed(hit: IVec3, within: Vec3) -> Vec3 {
        block_corner(hit) + within * BLOCK_SIZE
    }

    #[test]
    fn the_break_cube_holds_the_hit_block() {
        let hit = IVec3::new(3, -2, 7);
        for normal in [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z] {
            for within in [Vec3::splat(0.2), Vec3::splat(0.8), Vec3::new(0.2, 0.8, 0.5)] {
                let corner = break_corner(hit, aimed(hit, within), normal);
                assert!(break_cube(corner).any(|pos| pos == hit), "{normal} {within}");
            }
        }
    }

    #[test]
    fn the_break_cube_goes_into_the_surface_and_toward_the_aim() {
        let hit = IVec3::ZERO;
        // looking down at a floor (its top face points up), at the far +x, near
        // -z part of the block: the cube goes down, +x and -z
        let corner = break_corner(hit, aimed(hit, Vec3::new(0.9, 1.0, 0.1)), Vec3::Y);
        assert_eq!(corner, IVec3::new(0, -1, -1));
        // a wall facing -z, aimed at its upper left (low x): in is +z
        let corner = break_corner(hit, aimed(hit, Vec3::new(0.1, 0.9, 0.0)), Vec3::NEG_Z);
        assert_eq!(corner, IVec3::new(-1, 0, 0));
    }

    #[test]
    fn a_break_cube_is_the_whole_cube() {
        let cube: Vec<IVec3> = break_cube(IVec3::ONE).collect();
        assert_eq!(cube.len(), (BREAK_SIZE * BREAK_SIZE * BREAK_SIZE) as usize);
        assert!(cube.contains(&IVec3::ONE));
        assert!(cube.contains(&(IVec3::ONE + IVec3::splat(BREAK_SIZE - 1))));
    }
}
