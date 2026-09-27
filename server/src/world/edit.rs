use amity_protocol::{
    player::{aim, definition::PlayerMarker},
    world::{
        blocks::Blocks,
        chunk::ChunkPos,
        edit::edit_allowed,
        messages::{BlockChanged, EditBlock, WorldChannel},
    },
};
use avian3d::prelude::{Position, SpatialQuery};
use bevy::prelude::*;
use lightyear::connection::{client::Connected, client_of::ClientOf};
use lightyear::prelude::{ControlledBy, MessageReceiver, MessageSender};

use super::streaming::logic::ChunkView;

/// Checks and applies clients' block edits. Accepted edits go to every client
/// that has the chunk; a rejected edit gets the real block sent back, so that
/// client's view is corrected.
pub fn receive_edits(
    spatial: SpatialQuery,
    players: Query<(&Position, &ControlledBy), With<PlayerMarker>>,
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
) {
    let mut accepted = Vec::new();
    for (client, mut receiver, mut sender, _) in &mut clients {
        let eye = players
            .iter()
            .find(|(_, controlled)| controlled.owner == client)
            .map(|(position, _)| aim::eye(position.0));
        for edit in receiver.receive() {
            if eye.is_some_and(|eye| edit_allowed(&edit, eye, &spatial, &blocks)) {
                blocks.set(edit.pos, edit.block);
                accepted.push(BlockChanged {
                    pos: edit.pos,
                    block: edit.block,
                });
            } else if let Some(actual) = blocks.get(edit.pos) {
                sender.send::<WorldChannel>(BlockChanged {
                    pos: edit.pos,
                    block: actual,
                });
            }
        }
    }
    // clients without the chunk skip it: they get the edit with the chunk itself
    for (_, _, mut sender, view) in &mut clients {
        for change in &accepted {
            if view.has(ChunkPos::of_block(change.pos).0) {
                sender.send::<WorldChannel>(change.clone());
            }
        }
    }
}
