use amity_protocol::{
    player::definition::PlayerMarker,
    world::{
        chunk::{Chunk, ChunkIndex, ChunkPos},
        messages::{ChunkData, UnloadChunk, WorldChannel},
    },
};
use avian3d::prelude::Position;
use bevy::{platform::collections::HashSet, prelude::*};
use lightyear::connection::{client::Connected, client_of::ClientOf};
use lightyear::prelude::{ControlledBy, MessageSender};

use super::config::{SEND_BYTES_PER_TICK, UNLOAD_MARGIN, VIEW_DISTANCE};
use crate::world::area;

/// Message framing added to each chunk's own size when counting the budget
const MESSAGE_OVERHEAD: usize = 16;

/// Which chunks one client has. Lives on the server's entity for that client
#[derive(Component, Default)]
pub struct ChunkView {
    /// chunks sent and not since unloaded
    sent: HashSet<IVec3>,
    /// the chunk the player was in last tick
    center: Option<IVec3>,
    /// nothing left to send until the player moves or new chunks are generated
    complete: bool,
}

impl ChunkView {
    pub fn has(&self, pos: ChunkPos) -> bool {
        self.sent.contains(&pos.0)
    }
}

pub fn add_chunk_view(
    trigger: On<Add, Connected>,
    clients: Query<(), With<ClientOf>>,
    mut cmds: Commands,
) {
    if clients.contains(trigger.entity) {
        cmds.entity(trigger.entity).insert(ChunkView::default());
    }
}

/// Per client: when its player enters a new chunk, unload what fell out of range;
/// then send the nearest chunks it doesn't have yet, until the byte budget runs out.
/// Chunks that aren't generated yet are skipped and picked up once they are.
pub fn stream_chunks(
    players: Query<(&Position, &ControlledBy), With<PlayerMarker>>,
    mut clients: Query<
        (
            Entity,
            &mut ChunkView,
            &mut MessageSender<ChunkData>,
            &mut MessageSender<UnloadChunk>,
        ),
        (With<ClientOf>, With<Connected>),
    >,
    chunks: Query<&Chunk>,
    generated: Query<(), Added<Chunk>>,
    index: Res<ChunkIndex>,
    mut offsets: Local<Vec<IVec3>>,
) {
    if offsets.is_empty() {
        *offsets = area::nearest_first(VIEW_DISTANCE);
    }
    let new_chunks = !generated.is_empty();

    for (client, mut view, mut chunk_sender, mut unload_sender) in &mut clients {
        let Some(position) = players
            .iter()
            .find(|(_, controlled)| controlled.owner == client)
            .map(|(position, _)| position.0)
        else {
            continue;
        };

        let center = ChunkPos::containing(position).0;
        if view.center != Some(center) {
            view.center = Some(center);
            view.complete = false;
            let keep = VIEW_DISTANCE + IVec3::splat(UNLOAD_MARGIN);
            view.sent.retain(|&pos| {
                let inside = area::within(pos - center, keep);
                if !inside {
                    unload_sender.send::<WorldChannel>(UnloadChunk { pos: ChunkPos(pos) });
                }
                inside
            });
        }
        if new_chunks {
            view.complete = false;
        }
        if view.complete {
            continue;
        }

        let mut complete = true;
        let mut bytes = 0;
        for offset in offsets.iter() {
            let pos = center + *offset;
            if view.sent.contains(&pos) {
                continue;
            }
            let Some(chunk) = index.get(ChunkPos(pos)).and_then(|entity| chunks.get(entity).ok())
            else {
                complete = false; // not generated yet
                continue;
            };
            if bytes >= SEND_BYTES_PER_TICK {
                complete = false;
                break;
            }
            let chunk = chunk.compacted();
            bytes += chunk.byte_size() + MESSAGE_OVERHEAD;
            chunk_sender.send::<WorldChannel>(ChunkData {
                pos: ChunkPos(pos),
                chunk,
            });
            view.sent.insert(pos);
        }
        view.complete = complete;
    }
}
