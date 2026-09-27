//! Item types. Every block is an item (placing it places the block), and
//! `assets/items.ron` lists the rest; `protocol/build.rs` turns both into the
//! `Item::NAME` constants and the `ITEM_DEFS` table below.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::world::block::Block;

/// An item type, by id: blocks first (in blocks.ron order, without air), then
/// items.ron's. Name them in code with the generated constants: `Item::STONE`
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Item(u16);

/// One item's properties
struct ItemDef {
    name: &'static str,
    /// how many fit in one inventory slot
    stack: u32,
    /// the texture shown for it (file name without .png)
    icon: &'static str,
    /// the block it places, for block items
    places: Option<Block>,
}

// the generated `impl Item { pub const STONE ... }` and `const ITEM_DEFS`
include!(concat!(env!("OUT_DIR"), "/items.rs"));

impl Item {
    /// Every item, blocks first
    pub fn all() -> impl Iterator<Item = Item> {
        (0..ITEM_DEFS.len() as u16).map(Item)
    }

    /// The item for a block, if it has one (air doesn't)
    pub fn of_block(block: Block) -> Option<Item> {
        // block items come first, in block order, without air (block id 0)
        (block != Block::AIR).then(|| Item(block.id() - 1))
    }

    fn def(self) -> &'static ItemDef {
        &ITEM_DEFS[self.0 as usize]
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    /// How many fit in one inventory slot
    pub fn max_stack(self) -> u32 {
        self.def().stack
    }

    /// The texture shown for it in the inventory (file name without .png)
    pub fn icon(self) -> &'static str {
        self.def().icon
    }

    /// The block it places, if it's a block item
    pub fn places(self) -> Option<Block> {
        self.def().places
    }
}

/// Prints the name, e.g. `stone`, instead of `Item(0)`
impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_block_is_an_item() {
        for block in Block::all().filter(|&block| block != Block::AIR) {
            let item = Item::of_block(block).unwrap();
            assert_eq!(item.places(), Some(block));
            assert_eq!(item.name(), block.name());
        }
        assert_eq!(Item::of_block(Block::AIR), None);
        assert_eq!(Item::STONE.places(), Some(Block::STONE));
    }
}
