use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{config::HOTBAR_SLOTS, item::Item};

/// Some amount of one item, as held in an inventory slot
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: Item,
    pub count: u32,
}

/// The items a player holds, one stack per slot. Owned by the server and
/// replicated; the selected slot is `PlayerInputs::slot`
#[derive(Component, Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Hotbar {
    pub slots: [Option<ItemStack>; HOTBAR_SLOTS],
}

impl Hotbar {
    /// The stack in `slot`, if any (out-of-range slots are empty)
    pub fn get(&self, slot: usize) -> Option<ItemStack> {
        self.slots.get(slot).copied().flatten()
    }

    /// Puts `count` of `item` in: first onto stacks of it that aren't full, then
    /// into empty slots. Returns how many didn't fit
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

/// Client -> server: put `item` (a full stack) in hotbar `slot`, or empty it.
/// Only allowed in creative
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetSlot {
    pub slot: usize,
    pub item: Option<Item>,
}

/// Client -> server: start or stop flying. Only allowed in creative
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToggleFlight;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_fills_stacks_then_empty_slots() {
        let mut hotbar = Hotbar::default();
        hotbar.slots[1] = Some(ItemStack {
            item: Item::DIRT,
            count: 60,
        });
        // 4 fit on the dirt stack, 64 in the first empty slot, 2 in the next
        assert_eq!(hotbar.add(Item::DIRT, 70), 0);
        assert_eq!(hotbar.get(1).unwrap().count, 64);
        assert_eq!(hotbar.get(0).unwrap().count, 64);
        assert_eq!(hotbar.get(2).unwrap().count, 2);
        // full: what doesn't fit comes back
        assert_eq!(hotbar.add(Item::STONE, 64 * 3), 64);
    }

    #[test]
    fn taking_the_last_one_empties_the_slot() {
        let mut hotbar = Hotbar::default();
        hotbar.add(Item::SAND, 1);
        assert!(hotbar.take_one(0));
        assert_eq!(hotbar.get(0), None);
        assert!(!hotbar.take_one(0));
    }
}
