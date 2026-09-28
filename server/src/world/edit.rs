use amity_protocol::{
    inventory::{
        config::HOTBAR_SLOTS,
        definition::{GameMode, Inventory},
        item::Item,
    },
    player::{
        aim,
        definition::{PlayerInputs, PlayerMarker},
    },
    world::{
        block::Block,
        blocks::Blocks,
        chunk::ChunkPos,
        edit::edit_allowed,
        messages::{BlockChanged, BlockChannel, EditBlock},
    },
};
use avian3d::prelude::{Position, SpatialQuery};
use bevy::prelude::*;
use lightyear::connection::{client::Connected, client_of::ClientOf};
use lightyear::prelude::{input::native::ActionState, ControlledBy, MessageReceiver, MessageSender};

use super::{sky::SkyHeights, streaming::logic::ChunkView};

/// Checks and applies clients' block edits. Accepted edits go to every client
/// that has the chunk; a rejected edit gets the real block sent back, so that
/// client's view is corrected.
///
/// Placing needs the selected hotbar slot to hold the block's item; in survival
/// it uses one up, and breaking puts the block's item in the inventory (if
/// there's room). Creative players place anything they've picked, endlessly.
pub fn receive_edits(
    spatial: SpatialQuery,
    mut players: Query<
        (
            &Position,
            &ControlledBy,
            &GameMode,
            &mut Inventory,
            &ActionState<PlayerInputs>,
        ),
        With<PlayerMarker>,
    >,
    mut clients: Query<
        (
            Entity,
            &mut MessageReceiver<EditBlock>,
            &mut MessageSender<BlockChanged>,
            &ChunkView,
        ),
        (With<ClientOf>, With<Connected>),
    >,
    mut blocks: Blocks,
    mut sky: ResMut<SkyHeights>,
) {
    let mut accepted = Vec::new();
    for (client, mut receiver, mut sender, _) in &mut clients {
        let Some((position, _, mode, mut inventory, action)) = players
            .iter_mut()
            .find(|(_, controlled, ..)| controlled.owner == client)
        else {
            receiver.receive().for_each(drop);
            continue;
        };
        let eye = aim::eye(position.0);
        let survival = *mode == GameMode::Survival;
        // the selected hotbar slot (the client can't pick one past the hotbar)
        let slot = (action.0.slot as usize).min(HOTBAR_SLOTS - 1);
        for edit in receiver.receive() {
            let before = blocks.get(edit.pos);
            // placing: the selected item must place this block
            let holding = edit.block == Block::AIR
                || inventory
                    .get(slot)
                    .is_some_and(|stack| stack.item.places() == Some(edit.block));
            if holding && edit_allowed(&edit, eye, &spatial, &blocks) {
                if survival {
                    if edit.block == Block::AIR {
                        // what was broken goes in the inventory, if there's room
                        if let Some(item) = before.and_then(Item::of_block) {
                            inventory.add(item, 1);
                        }
                    } else {
                        inventory.take_one(slot);
                    }
                }
                blocks.set(edit.pos, edit.block);
                accepted.push(BlockChanged {
                    pos: edit.pos,
                    block: edit.block,
                    sky: sky.edited(edit.pos, edit.block, &blocks),
                });
            } else if let Some(actual) = blocks.get(edit.pos) {
                sender.send::<BlockChannel>(BlockChanged {
                    pos: edit.pos,
                    block: actual,
                    sky: None,
                });
            }
        }
    }
    // clients without the chunk skip it: they get the edit with the chunk itself
    for (_, _, mut sender, view) in &mut clients {
        for change in &accepted {
            if view.has(ChunkPos::of_block(change.pos).0) {
                sender.send::<BlockChannel>(change.clone());
            }
        }
    }
}
