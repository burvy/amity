use amity_protocol::{
    inventory::definition::Inventory,
    player::definition::PlayerInputs,
    world::{
        block::Block,
        blocks::Blocks,
        chunk::ColumnSky,
        chunk::{ChunkIndex, ChunkPos},
        edit::{player_edit, EditAction},
        messages::{BlockChanged, EditBlock, EditChannel},
    },
};
use avian3d::prelude::*;
use bevy::{platform::collections::HashMap, prelude::*};
use lightyear::prelude::{input::native::ActionState, Controlled, MessageReceiver, MessageSender};

use super::{chunks::IncomingChunks, sky::Skies};

/// Asks the server to make our player's edits. Nothing changes locally until the
/// server's `BlockChanged` arrives, so the client's world is only ever the server's.
/// Reads this tick's inputs (empty while paused), and runs per tick, so holding a
/// button edits at the tick rate whatever the FPS.
///
/// - Breaking: every tick the button is held, so dragging across blocks breaks
///   them all. Until the answer arrives the same break is re-sent each tick; the
///   server rejects the duplicates harmlessly
/// - Placing: one block per click. Holding sprint (Shift) as well places every
///   tick, for building walls and bridges quickly
pub fn request_edits(
    spatial: SpatialQuery,
    player: Single<
        (Entity, &Position, &ActionState<PlayerInputs>, Option<&Inventory>),
        With<Controlled>,
    >,
    blocks: Blocks,
    mut sender: Single<&mut MessageSender<EditBlock>>,
    mut was_placing: Local<bool>,
) {
    let (player, position, action, inventory) = player.into_inner();
    let input = &action.0;
    // a new click: place was up last tick and is down now
    let click = input.secondary && !*was_placing;
    *was_placing = input.secondary;
    // the block the selected slot's item places, if it places one
    let place = inventory
        .and_then(|inventory| inventory.get(input.slot as usize))
        .and_then(|stack| stack.item.places());
    // break wins when both are held
    let edit_action = if input.primary {
        EditAction::Break
    } else if input.secondary && (click || input.sprint) {
        EditAction::Place
    } else {
        return;
    };
    let Some((pos, block)) =
        player_edit(edit_action, place, player, position.0, input.look, &spatial, &blocks)
    else {
        return;
    };
    sender.send::<EditChannel>(EditBlock { pos, block });
}

/// Block changes for chunks that haven't arrived yet, by chunk position, oldest
/// first. Changes come on their own channel, so they can overtake their chunk
/// (which may be queued behind many others). The server only sends a change for
/// a chunk it has already sent, so the chunk, when it arrives, is from before the
/// change, and applying the held changes on top of it is correct
#[derive(Resource, Default)]
pub struct HeldChanges(HashMap<IVec3, Vec<(IVec3, Block, Option<ColumnSky>)>>);

/// The server's block changes: the only way the client's world changes after a
/// chunk arrives. Changes for chunks not here yet are held until they are, as are
/// changes for a chunk whose newer copy is still on its way in (applying them to
/// the old copy would be lost when the new one replaces it)
pub fn receive_block_changes(
    mut receiver: Single<&mut MessageReceiver<BlockChanged>>,
    mut held: ResMut<HeldChanges>,
    index: Res<ChunkIndex>,
    incoming: Res<IncomingChunks>,
    mut blocks: Blocks,
    mut skies: Skies,
) {
    let ready = |chunk: ChunkPos| index.contains(chunk) && !incoming.contains(chunk);
    // first, changes held for chunks that have arrived since
    held.0.retain(|&chunk, changes| {
        if !ready(ChunkPos(chunk)) {
            return true;
        }
        for &(pos, block, sky) in changes.iter() {
            blocks.set(pos, block);
            if let Some(sky) = sky {
                skies.column_changed(pos, sky);
            }
        }
        false
    });
    for BlockChanged { pos, block, sky } in receiver.receive() {
        // a changed column height reaches the column's chunks that are here now.
        // A copy of this chunk from before the change may still be on its way:
        // the height is set on it again when the held change is applied
        if let Some(sky) = sky {
            skies.column_changed(pos, sky);
        }
        let chunk = ChunkPos::of_block(pos).0;
        if ready(chunk) {
            blocks.set(pos, block);
        } else {
            held.0.entry(chunk.0).or_default().push((pos, block, sky));
        }
    }
}
