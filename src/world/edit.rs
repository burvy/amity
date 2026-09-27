use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::{
    input::native::ActionState, Controlled, MessageReceiver, MessageSender,
};
use shooter_protocol::{
    protocol::PlayerInputs,
    world::edit::{self, BlockChanged, Blocks, EditAction, EditBlock, EditChannel},
};

/// Applies our own player's edits immediately and sends them to the server to
/// check. Reads this tick's inputs (empty while paused), and runs per tick, so
/// holding a button edits at the tick rate whatever the FPS
pub fn predict_edits(
    spatial: SpatialQuery,
    player: Single<(Entity, &Position, &ActionState<PlayerInputs>), With<Controlled>>,
    mut blocks: Blocks,
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
    if blocks.set(pos, block) {
        sender.send::<EditChannel>(EditBlock { pos, block });
    }
}

/// The server's block changes are the truth. The server answers every edit we
/// send, with the new block if accepted or the real one if rejected, so this also
/// corrects any wrong prediction.
pub fn receive_block_changes(
    mut receiver: Single<&mut MessageReceiver<BlockChanged>>,
    mut blocks: Blocks,
) {
    for BlockChanged { pos, block } in receiver.receive() {
        blocks.set(pos, block);
    }
}
