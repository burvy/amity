//! Block types, and converting between metres and block coordinates

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::config::BLOCK_SIZE;

/// To add a block: add it here, give it a texture in client
/// `world/meshing/textures.rs`, and drop its png in assets/images/blocks
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum Block {
    #[default]
    Air,
    Stone,
    Dirt,
    Grass,
}

impl Block {
    pub fn is_solid(self) -> bool {
        self != Block::Air
    }
}

/// World position in metres of a block's minimum corner
pub fn block_corner(pos: IVec3) -> Vec3 {
    pos.as_vec3() * BLOCK_SIZE
}

/// The block containing a world point in metres
pub fn block_at(point: Vec3) -> IVec3 {
    (point / BLOCK_SIZE).floor().as_ivec3()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metres_convert_to_blocks() {
        assert_eq!(block_at(Vec3::new(-0.01, 0.0, BLOCK_SIZE - 0.01)), IVec3::new(-1, 0, 0));
        assert_eq!(block_at(Vec3::splat(BLOCK_SIZE)), IVec3::ONE);
        assert_eq!(block_at(block_corner(IVec3::new(-3, 7, 0))), IVec3::new(-3, 7, 0));
    }
}
