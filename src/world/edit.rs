use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::{
    core::tick::Tick,
    prelude::{input::native::ActionState, Controlled, LocalTimeline, MessageReceiver},
};
use shooter_protocol::{
    protocol,
    world::{
        chunk::Block,
        edit::{self, BlockChanged, Blocks},
    },
};

/// reverts predictions past this amount of ticks
/// TODO: scale to ping
const CONFIRM_TIMEOUT_TICKS: i32 = 64;

/// A predicted edit the server hasn't settled yet
struct PendingEdit {
    pos: IVec3,
    /// the block before our prediction, restored if the server never confirms
    previous: Block,
    tick: Tick,
}

/// Predicted edits waiting for the server
#[derive(Resource, Default)]
pub struct PendingEdits(Vec<PendingEdit>);

/// Applies our own player's edits immediately, using the same function as the server
pub fn predict_edits(
    spatial: SpatialQuery,
    player: Single<
        (Entity, &Position, &ActionState<protocol::PlayerInputs>),
        With<Controlled>,
    >,
    mut blocks: Blocks,
    mut pending: ResMut<PendingEdits>,
    timeline: Res<LocalTimeline>,
) {
    let (player, position, action) = player.into_inner();
    let Some((pos, block)) = edit::player_edit(&action.0, player, position.0, &spatial, &blocks)
    else {
        return;
    };
    let Some(previous) = blocks.get(pos) else {
        return;
    };
    if blocks.set(pos, block) {
        let tick = timeline.tick();
        match pending.0.iter_mut().find(|edit| edit.pos == pos) {
            // keep the original `previous`: that's what the server still has
            Some(edit) => edit.tick = tick,
            None => pending.0.push(PendingEdit {
                pos,
                previous,
                tick,
            }),
        }
    }
}

/// The server's block changes are the truth, and settle any prediction at that position
pub fn receive_block_changes(
    mut receiver: Single<&mut MessageReceiver<BlockChanged>>,
    mut blocks: Blocks,
    mut pending: ResMut<PendingEdits>,
) {
    for BlockChanged { pos, block } in receiver.receive() {
        blocks.set(pos, block);
        pending.0.retain(|edit| edit.pos != pos);
    }
}

/// Undoes predictions the server never confirmed
pub fn expire_pending_edits(
    mut blocks: Blocks,
    mut pending: ResMut<PendingEdits>,
    timeline: Res<LocalTimeline>,
) {
    let now = timeline.tick();
    pending.0.retain(|edit| {
        let expired = now - edit.tick > CONFIRM_TIMEOUT_TICKS;
        if expired {
            blocks.set(edit.pos, edit.previous);
        }
        !expired
    });
}
