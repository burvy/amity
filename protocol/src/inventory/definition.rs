use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    config::{HOTBAR_SLOTS, INVENTORY_SLOTS},
    item::Item,
};

/// Some amount of one item, as held in an inventory slot
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: Item,
    pub count: u32,
}

/// The items a player holds. Owned by the server and replicated.
///
/// Slots `0..HOTBAR_SLOTS` are the hotbar (selected with `PlayerInputs::slot`);
/// the rest are the inventory that slides out with Tab. `cursor` is the stack
/// picked up with the mouse while the inventory is open: it's part of the
/// inventory, so the server always knows where every item is
#[derive(Component, Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: [Option<ItemStack>; INVENTORY_SLOTS],
    pub cursor: Option<ItemStack>,
}

/// Which mouse button clicked a slot
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Click {
    /// Pick up the whole stack, put the whole held stack down, merge, or swap
    Left,
    /// Pick up half the stack, or put one of the held items down
    Right,
}

impl Inventory {
    /// The stack in `slot`, if any (out-of-range slots are empty)
    pub fn get(&self, slot: usize) -> Option<ItemStack> {
        self.slots.get(slot).copied().flatten()
    }

    /// Puts `count` of `item` in: first onto stacks of it that aren't full, then
    /// into empty slots, hotbar first. Returns how many didn't fit
    pub fn add(&mut self, item: Item, mut count: u32) -> u32 {
        let max = item.max_stack();
        for stack in self.slots.iter_mut().flatten() {
            if stack.item == item && stack.count < max {
                let moved = count.min(max - stack.count);
                stack.count += moved;
                count -= moved;
            }
        }
        for slot in self.slots.iter_mut().filter(|slot| slot.is_none()) {
            if count == 0 {
                break;
            }
            let moved = count.min(max);
            *slot = Some(ItemStack { item, count: moved });
            count -= moved;
        }
        count
    }

    /// Takes one item from `slot`, emptying it when it runs out. False if the
    /// slot was empty
    pub fn take_one(&mut self, slot: usize) -> bool {
        let Some(Some(stack)) = self.slots.get_mut(slot) else {
            return false;
        };
        stack.count -= 1;
        if stack.count == 0 {
            self.slots[slot] = None;
        }
        true
    }

    /// A click on `slot` with the inventory open, moving items between the slot
    /// and the cursor:
    /// left click picks up and puts down a stack
    /// right click picks half or puts one down
    pub fn click(&mut self, slot: usize, click: Click) {
        let Some(target) = self.slots.get_mut(slot) else {
            return;
        };
        match (click, self.cursor.take(), target.take()) {
            (_, None, None) => {}
            // picking up
            (Click::Left, None, Some(stack)) => self.cursor = Some(stack),
            (Click::Right, None, Some(stack)) => {
                let taken = stack.count.div_ceil(2);
                self.cursor = Some(ItemStack {
                    count: taken,
                    ..stack
                });
                *target = (stack.count > taken).then_some(ItemStack {
                    count: stack.count - taken,
                    ..stack
                });
            }
            // putting down onto an empty slot
            (Click::Left, Some(held), None) => *target = Some(held),
            (Click::Right, Some(held), None) => {
                *target = Some(ItemStack { count: 1, ..held });
                self.cursor = (held.count > 1).then_some(ItemStack {
                    count: held.count - 1,
                    ..held
                });
            }
            // onto the same item: merge as much as fits
            (click, Some(held), Some(stack)) if held.item == stack.item => {
                let room = stack.item.max_stack() - stack.count;
                let wanted = if click == Click::Left { held.count } else { 1 };
                let moved = wanted.min(room);
                *target = Some(ItemStack {
                    count: stack.count + moved,
                    ..stack
                });
                self.cursor = (held.count > moved).then_some(ItemStack {
                    count: held.count - moved,
                    ..held
                });
            }
            // onto another item: swap (both buttons)
            (_, Some(held), Some(stack)) => {
                *target = Some(held);
                self.cursor = Some(stack);
            }
        }
    }

    /// Closing the inventory: whatever is on the cursor goes back into the slots.
    /// Anything that doesn't fit stays on the cursor (until there are dropped items)
    pub fn put_back_cursor(&mut self) {
        if let Some(held) = self.cursor.take() {
            let left = self.add(held.item, held.count);
            self.cursor = (left > 0).then_some(ItemStack {
                count: left,
                ..held
            });
        }
    }

    /// Whether `slot` is on the hotbar
    pub fn is_hotbar(slot: usize) -> bool {
        slot < HOTBAR_SLOTS
    }
}

/// How a player plays. Set by the server and replicated
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum GameMode {
    /// Items are used up when placed, and blocks broken become items
    Survival,
    /// Any item, unlimited, and flying
    Creative,
}

/// Whether a player is flying. Replicated and predicted, since movement depends
/// on it; only the server changes it (see `ToggleFlight`)
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Flying(pub bool);

/// Client -> server, ordered + reliable: one-off player actions that must not be
/// lost or guessed the way per-tick inputs can be
pub struct ActionChannel;

/// Client -> server: something done in the inventory. One message type, so the
/// server applies them in exactly the order they were done (picking a block and
/// then clicking a slot must not become the other way round)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum InventoryAction {
    /// A click on an inventory slot (see `Inventory::click`)
    Click { slot: usize, click: Click },
    /// Creative only: put a full stack of `item` on the cursor (from the block
    /// picker), or throw away what's on it (`None`)
    Pick { item: Option<Item> },
    /// The inventory was closed: the cursor's items go back into the slots
    Close,
}

/// Client -> server: start or stop flying. Only allowed in creative
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToggleFlight;

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: Item, count: u32) -> Option<ItemStack> {
        Some(ItemStack { item, count })
    }

    #[test]
    fn adding_fills_stacks_then_empty_slots() {
        let mut inventory = Inventory::default();
        inventory.slots[1] = stack(Item::DIRT, 60);
        // 4 fit on the dirt stack, 64 in the first empty slot, 2 in the next
        assert_eq!(inventory.add(Item::DIRT, 70), 0);
        assert_eq!(inventory.get(1).unwrap().count, 64);
        assert_eq!(inventory.get(0).unwrap().count, 64);
        assert_eq!(inventory.get(2).unwrap().count, 2);
    }

    #[test]
    fn taking_the_last_one_empties_the_slot() {
        let mut inventory = Inventory::default();
        inventory.add(Item::SAND, 1);
        assert!(inventory.take_one(0));
        assert_eq!(inventory.get(0), None);
        assert!(!inventory.take_one(0));
    }

    #[test]
    fn left_click_picks_up_puts_down_and_swaps() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = stack(Item::STONE, 10);
        inventory.slots[1] = stack(Item::DIRT, 3);
        inventory.click(0, Click::Left);
        assert_eq!(
            (inventory.get(0), inventory.cursor),
            (None, stack(Item::STONE, 10))
        );
        inventory.click(1, Click::Left);
        assert_eq!(
            (inventory.get(1), inventory.cursor),
            (stack(Item::STONE, 10), stack(Item::DIRT, 3))
        );
        inventory.click(0, Click::Left);
        assert_eq!(
            (inventory.get(0), inventory.cursor),
            (stack(Item::DIRT, 3), None)
        );
    }

    #[test]
    fn right_click_splits_and_places_one() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = stack(Item::SAND, 5);
        inventory.click(0, Click::Right);
        assert_eq!(
            (inventory.get(0), inventory.cursor),
            (stack(Item::SAND, 2), stack(Item::SAND, 3))
        );
        inventory.click(7, Click::Right);
        assert_eq!(
            (inventory.get(7), inventory.cursor),
            (stack(Item::SAND, 1), stack(Item::SAND, 2))
        );
    }

    #[test]
    fn merging_stops_at_a_full_stack() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = stack(Item::STONE, 60);
        inventory.cursor = stack(Item::STONE, 10);
        inventory.click(0, Click::Left);
        assert_eq!(
            (inventory.get(0), inventory.cursor),
            (stack(Item::STONE, 64), stack(Item::STONE, 6))
        );
    }

    #[test]
    fn closing_puts_the_cursor_back() {
        let mut inventory = Inventory {
            cursor: stack(Item::GRAVEL, 7),
            ..default()
        };
        inventory.put_back_cursor();
        assert_eq!(
            (inventory.get(0), inventory.cursor),
            (stack(Item::GRAVEL, 7), None)
        );
    }
}
