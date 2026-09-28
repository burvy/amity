//! Block types, and converting between metres and block coordinates.
//!
//! The blocks themselves are listed in `assets/blocks.ron`; `protocol/build.rs`
//! turns that list into the `Block::NAME` constants and the `DEFS` table below.

use std::fmt;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::config::BLOCK_SIZE;

/// A block type, by id: its position in blocks.ron, with air as 0. Name them in
/// code with the generated constants: `Block::AIR`, `Block::STONE`, ...
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Block(u16);

/// One block's properties, as written in blocks.ron
struct BlockDef {
    name: &'static str,
    solid: bool,
    fluid: bool,
    see_through: bool,
    /// texture file names (no .png) per face
    top: &'static str,
    side: &'static str,
    bottom: &'static str,
}

// the generated `impl Block { pub const AIR ... }` and `const DEFS`
include!(concat!(env!("OUT_DIR"), "/blocks.rs"));

impl Block {
    /// Every block, air first
    pub fn all() -> impl Iterator<Item = Block> {
        (0..DEFS.len() as u16).map(Block)
    }

    /// The id: position in blocks.ron, air 0. For tables indexed by block
    pub fn id(self) -> u16 {
        self.0
    }

    fn def(self) -> &'static BlockDef {
        &DEFS[self.0 as usize]
    }

    /// The block's name in blocks.ron
    pub fn name(self) -> &'static str {
        self.def().name
    }

    /// The block with this name in blocks.ron (or "air"), if there is one
    pub fn named(name: &str) -> Option<Block> {
        Block::all().find(|block| block.name() == name)
    }

    /// Whether players collide with it, and whether it hides its neighbours' faces
    pub fn is_solid(self) -> bool {
        self.def().solid
    }

    /// Water and the like: not solid, drawn see-through, and a block can be
    /// placed into it
    pub fn is_fluid(self) -> bool {
        self.def().fluid
    }

    /// Solid, but with gaps, like leaves: you see through its transparent
    /// pixels, and sky light passes through it
    pub fn is_see_through(self) -> bool {
        self.def().see_through
    }

    /// Solid all the way through: hides the faces of blocks behind it, and
    /// stops sky light. Solid blocks that aren't see-through
    pub fn is_opaque(self) -> bool {
        self.is_solid() && !self.is_see_through()
    }

    /// The texture file name (no .png) on the face pointing along `normal`
    pub fn texture(self, normal: IVec3) -> &'static str {
        let def = self.def();
        match normal {
            IVec3::Y => def.top,
            IVec3::NEG_Y => def.bottom,
            _ => def.side,
        }
    }
}

/// Prints the name, e.g. `stone`, instead of `Block(1)`
impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
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
