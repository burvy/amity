use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::{
    input::native::ActionState, Controlled, MessageReceiver, MessageSender,
};
use shooter_protocol::{
    protocol::PlayerInputs,
    world::edit::{self, BlockChanged, Blocks, EditAction, EditBlock, EditChannel},
};

/// Asks the server to make our player's edits. Nothing changes locally until the
/// server's `BlockChanged` arrives, so the client's world is only ever the server's.
/// Reads this tick's inputs (empty while paused), and runs per tick, so holding a
/// button edits at the tick rate whatever the FPS. Until the answer arrives the
/// same edit is re-sent each tick; the server rejects the duplicates harmlessly.
pub fn request_edits(
    spatial: SpatialQuery,
    player: Single<(Entity, &Position, &ActionState<PlayerInputs>), With<Controlled>>,
    blocks: Blocks,
    mut sender: Single<&mut MessageSender<EditBlock>>,
) {
    let (player, position, action) = player.into_inner();
    let input = &action.0;
    // break wins when both are held
    let edit_action = if input.primary {
        EditAction::Break
    } else if input.secondary {
        EditAction::Place
    } else {
        return;
    };
    let Some((pos, block)) =
        edit::player_edit(edit_action, player, position.0, input.look, &spatial, &blocks)
    else {
        return;
    };
    sender.send::<EditChannel>(EditBlock { pos, block });
}

/// The server's block changes: the only way the client's world changes after a
/// chunk arrives
pub fn receive_block_changes(
    mut receiver: Single<&mut MessageReceiver<BlockChanged>>,
    mut blocks: Blocks,
) {
    for BlockChanged { pos, block } in receiver.receive() {
        blocks.set(pos, block);
    }
}
