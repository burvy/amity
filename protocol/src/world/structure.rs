//! Structures: boxes of blocks, like trees, that world generation places. Built
//! in game and captured to a file (the client's dev build: F6, F7, F8), read by
//! the server, which packs every file in `assets/structures` into itself.
//!
//! The file is RON, small enough to read:
//!
//! ```ron
//! (
//!     size: (5, 8, 5),
//!     palette: ["air", "grass", "oak_log", "oak_leaves"],
//!     blocks: [(1, 25), (0, 12), (2, 1), ...],
//! )
//! ```
//!
//! - `size`: blocks along x, y (up) and z
//! - `palette`: the block names the box uses. Names, not ids, so reordering
//!   `blocks.ron` never breaks a structure
//! - `blocks`: every block of the box as runs of (palette index, how many),
//!   x fastest, then z, then y from the bottom up
//!
//! Two rules for what's in the box:
//! - its bottom layer is the ground it stands on. It's never placed, it only
//!   shows where the ground goes. The layer above it is where the structure
//!   starts
//! - air means "leave the world as it is here", so a tree doesn't carve an
//!   empty box into a hillside around itself
//!
//! A structure is planted by its anchor, set on the top of the ground where it
//! goes: the middle of whatever stands on the ground (its lowest blocks, like a
//! trunk), so the box needn't be centred on it.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::block::Block;

/// A structure file, as written (see the module docs)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructureFile {
    pub size: (i32, i32, i32),
    pub palette: Vec<String>,
    pub blocks: Vec<(u16, u32)>,
}

/// A structure, ready to place
#[derive(Clone, Debug, PartialEq)]
pub struct Structure {
    /// blocks along x, y, z, the ground layer included
    size: IVec3,
    /// every block of the box, x fastest, then z, then y
    blocks: Vec<Block>,
}

/// A turn and flip for placing a structure: 3 files look like 24 structures
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Orientation {
    /// quarter turns about the vertical, 0 to 3
    pub turns: u8,
    /// mirrored along x first
    pub mirrored: bool,
}

impl Orientation {
    /// Turns and flips an offset from the anchor
    pub fn apply(self, offset: IVec3) -> IVec3 {
        let mut p = offset;
        if self.mirrored {
            p.x = -p.x;
        }
        for _ in 0..self.turns % 4 {
            // a quarter turn: x goes to z, z to -x
            p = IVec3::new(-p.z, p.y, p.x);
        }
        p
    }
}

impl Structure {
    /// A box of `size` blocks, all air
    pub fn empty(size: IVec3) -> Self {
        Self {
            size,
            blocks: vec![Block::AIR; (size.x * size.y * size.z).max(0) as usize],
        }
    }

    pub fn size(&self) -> IVec3 {
        self.size
    }

    fn index(&self, p: IVec3) -> usize {
        (p.x + p.z * self.size.x + p.y * self.size.x * self.size.z) as usize
    }

    /// The block at `p` in the box (0 to size - 1 on each axis)
    pub fn get(&self, p: IVec3) -> Block {
        self.blocks[self.index(p)]
    }

    pub fn set(&mut self, p: IVec3, block: Block) {
        let i = self.index(p);
        self.blocks[i] = block;
    }

    /// The anchor: where in the box sits on the ground's top. On the first
    /// layer above the ground, in the middle of the blocks in the lowest layer
    /// that has any (a trunk and its roots); the middle of the box if it's empty
    fn anchor(&self) -> IVec3 {
        let lowest = (1..self.size.y).find_map(|y| {
            let feet: Vec<IVec2> = (0..self.size.z)
                .flat_map(|z| (0..self.size.x).map(move |x| IVec2::new(x, z)))
                .filter(|p| self.get(IVec3::new(p.x, y, p.y)) != Block::AIR)
                .collect();
            (!feet.is_empty()).then(|| {
                let sum = feet.iter().fold(IVec2::ZERO, |sum, &p| sum + p);
                (sum.as_vec2() / feet.len() as f32).round().as_ivec2()
            })
        });
        let middle = lowest.unwrap_or(IVec2::new(self.size.x / 2, self.size.z / 2));
        IVec3::new(middle.x, 1, middle.y)
    }

    /// The most the structure reaches sideways from its anchor, in blocks, in
    /// any orientation
    pub fn reach(&self) -> i32 {
        let anchor = self.anchor();
        [anchor.x, self.size.x - 1 - anchor.x, anchor.z, self.size.z - 1 - anchor.z]
            .into_iter()
            .max()
            .unwrap_or(0)
    }

    /// How many blocks it rises above the ground
    pub fn height(&self) -> i32 {
        self.size.y - 1
    }

    /// The blocks it places, as offsets from the anchor, turned and flipped:
    /// everything but the ground layer and air
    pub fn placed(&self, orientation: Orientation) -> impl Iterator<Item = (IVec3, Block)> + '_ {
        let anchor = self.anchor();
        (1..self.size.y).flat_map(move |y| {
            (0..self.size.z).flat_map(move |z| {
                (0..self.size.x).filter_map(move |x| {
                    let p = IVec3::new(x, y, z);
                    let block = self.get(p);
                    (block != Block::AIR).then(|| (orientation.apply(p - anchor), block))
                })
            })
        })
    }

    /// Reads a structure file's text. Fails, saying why, on anything that
    /// doesn't fit: bad RON, an unknown block name, the wrong number of blocks
    pub fn parse(text: &str) -> Result<Self, String> {
        let file: StructureFile = ron::from_str(text).map_err(|e| e.to_string())?;
        let palette = file
            .palette
            .iter()
            .map(|name| Block::named(name).ok_or_else(|| format!("no block is named {name:?}")))
            .collect::<Result<Vec<Block>, String>>()?;
        let size = IVec3::new(file.size.0, file.size.1, file.size.2);
        if size.cmplt(IVec3::ONE).any() || size.y < 2 {
            return Err(format!(
                "size {size} is too small: at least 1 across, and 2 high (the ground layer and one above)"
            ));
        }
        let mut blocks = Vec::with_capacity((size.x * size.y * size.z) as usize);
        for &(index, count) in &file.blocks {
            let block = *palette
                .get(index as usize)
                .ok_or_else(|| format!("palette index {index} is past its end"))?;
            blocks.extend(std::iter::repeat_n(block, count as usize));
        }
        let expected = (size.x * size.y * size.z) as usize;
        if blocks.len() != expected {
            return Err(format!("{} blocks, but a {size} box has {expected}", blocks.len()));
        }
        Ok(Self { size, blocks })
    }

    /// The structure as a file, with runs of repeated blocks packed together
    pub fn to_file(&self) -> StructureFile {
        let mut palette: Vec<Block> = Vec::new();
        let mut runs: Vec<(u16, u32)> = Vec::new();
        for &block in &self.blocks {
            let index = match palette.iter().position(|&b| b == block) {
                Some(index) => index,
                None => {
                    palette.push(block);
                    palette.len() - 1
                }
            } as u16;
            match runs.last_mut() {
                Some((last, count)) if *last == index => *count += 1,
                _ => runs.push((index, 1)),
            }
        }
        StructureFile {
            size: (self.size.x, self.size.y, self.size.z),
            palette: palette.iter().map(|block| block.name().to_owned()).collect(),
            blocks: runs,
        }
    }

    /// The file's text, formatted for reading
    pub fn to_ron(&self) -> String {
        let config = ron::ser::PrettyConfig::new().depth_limit(1);
        ron::ser::to_string_pretty(&self.to_file(), config)
            .expect("a structure file is plain data, so it always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A little tree: a ground layer of grass, a 2-block trunk, a leaf on top
    fn tree() -> Structure {
        let mut tree = Structure::empty(IVec3::new(3, 4, 3));
        for x in 0..3 {
            for z in 0..3 {
                tree.set(IVec3::new(x, 0, z), Block::GRASS);
            }
        }
        tree.set(IVec3::new(1, 1, 1), Block::OAK_LOG);
        tree.set(IVec3::new(1, 2, 1), Block::OAK_LOG);
        tree.set(IVec3::new(1, 3, 1), Block::OAK_LEAVES);
        tree
    }

    #[test]
    fn a_structure_survives_its_file() {
        let tree = tree();
        assert_eq!(Structure::parse(&tree.to_ron()), Ok(tree));
    }

    #[test]
    fn a_file_names_its_problem() {
        let bad = r#"(size: (1, 2, 1), palette: ["air", "nonsense"], blocks: [(1, 2)])"#;
        assert!(Structure::parse(bad).unwrap_err().contains("nonsense"));
        let short = r#"(size: (1, 2, 1), palette: ["air"], blocks: [(0, 1)])"#;
        assert!(Structure::parse(short).is_err());
    }

    #[test]
    fn the_ground_layer_and_air_are_never_placed() {
        let placed: Vec<(IVec3, Block)> = tree().placed(Orientation::default()).collect();
        assert_eq!(
            placed,
            vec![
                (IVec3::new(0, 0, 0), Block::OAK_LOG),
                (IVec3::new(0, 1, 0), Block::OAK_LOG),
                (IVec3::new(0, 2, 0), Block::OAK_LEAVES),
            ]
        );
    }

    #[test]
    fn the_anchor_is_under_the_trunk_not_the_box_middle() {
        // a trunk off to one side of a wide box
        let mut tree = Structure::empty(IVec3::new(8, 3, 5));
        tree.set(IVec3::new(2, 1, 1), Block::OAK_LOG);
        tree.set(IVec3::new(2, 2, 1), Block::OAK_LOG);
        tree.set(IVec3::new(6, 2, 3), Block::OAK_LEAVES);
        let placed: Vec<(IVec3, Block)> = tree.placed(Orientation::default()).collect();
        assert_eq!(placed[0], (IVec3::ZERO, Block::OAK_LOG));
    }

    #[test]
    fn turning_four_times_or_flipping_twice_is_no_change() {
        let p = IVec3::new(2, 5, -1);
        let turned = (0..4).fold(p, |p, _| Orientation { turns: 1, mirrored: false }.apply(p));
        assert_eq!(turned, p);
        let flip = Orientation { turns: 0, mirrored: true };
        assert_eq!(flip.apply(flip.apply(p)), p);
        // a quarter turn keeps height and distance from the anchor
        let quarter = Orientation { turns: 1, mirrored: false }.apply(p);
        assert_eq!(quarter.y, p.y);
        assert_eq!(quarter.xz().length_squared(), p.xz().length_squared());
        assert_ne!(quarter, p);
    }
}
