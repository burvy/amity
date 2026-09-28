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
        edit::{break_cube, edit_allowed},
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
            // placing: the selected item must place this block
            let holding = match edit {
                EditBlock::Break { .. } => true,
                EditBlock::Place { block, .. } => inventory
                    .get(slot)
                    .is_some_and(|stack| stack.item.places() == Some(block)),
            };
            let allowed = holding && edit_allowed(&edit, eye, &spatial, &blocks);
            // the blocks this edit touches, and what each becomes
            let changes: Vec<(IVec3, Block)> = match edit {
                EditBlock::Break { corner } => break_cube(corner)
                    .filter(|&pos| blocks.get(pos).is_some_and(Block::is_solid))
                    .map(|pos| (pos, Block::AIR))
                    .collect(),
                EditBlock::Place { pos, block } => vec![(pos, block)],
            };
            if !allowed {
                // send back what's really there, correcting the client's view
                for (pos, _) in changes {
                    if let Some(actual) = blocks.get(pos) {
                        sender.send::<BlockChannel>(BlockChanged {
                            pos,
                            block: actual,
                            sky: None,
                        });
                    }
                }
                continue;
            }
            for (pos, block) in changes {
                if survival {
                    if block == Block::AIR {
                        // what was broken goes in the inventory, if there's room
                        if let Some(item) = blocks.get(pos).and_then(Item::of_block) {
                            inventory.add(item, 1);
                        }
                    } else {
                        inventory.take_one(slot);
                    }
                }
                blocks.set(pos, block);
                accepted.push(BlockChanged {
                    pos,
                    block,
                    sky: sky.edited(pos, block, &blocks),
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
