use amity_protocol::{
    player::definition::PlayerMarker,
    world::{
        chunk::{Chunk, ChunkIndex, ChunkPos},
        messages::{ChunkData, SetViewDistance, UnloadChunk, WorldChannel},
    },
};
use avian3d::prelude::Position;
use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use lightyear::connection::{client::Connected, client_of::ClientOf};
use lightyear::prelude::{ControlledBy, MessageReceiver, MessageSender};

use super::config::{SEND_BYTES_PER_TICK, UNLOAD_MARGIN, VIEW_DISTANCE};
use crate::world::{area, sky::SkyHeights};

/// Message framing added to each chunk's own size when counting the budget
const MESSAGE_OVERHEAD: usize = 16;

/// Which chunks one client has. Lives on the server's entity for that client
#[derive(Component, Default)]
pub struct ChunkView {
    /// chunks sent and not since unloaded
    sent: HashSet<IVec3>,
    /// how far out this client gets chunks: its render distance setting, capped at
    /// VIEW_DISTANCE (None until it says: VIEW_DISTANCE)
    distance: Option<IVec3>,
    /// the chunk the player was in last tick
    center: Option<IVec3>,
    /// nothing left to send until the player moves or new chunks are generated
    complete: bool,
}

impl ChunkView {
    pub fn has(&self, pos: ChunkPos) -> bool {
        self.sent.contains(&pos.0)
    }

    fn distance(&self) -> IVec3 {
        self.distance.unwrap_or(VIEW_DISTANCE)
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
    sky: Res<SkyHeights>,
    mut offsets: Local<HashMap<IVec3, Vec<IVec3>>>,
) {
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
            let keep = view.distance() + IVec3::splat(UNLOAD_MARGIN);
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
        // the box of chunk offsets for this client's distance, nearest first,
        // worked out once per distance
        let offsets = offsets
            .entry(view.distance())
            .or_insert_with_key(|&distance| area::nearest_first(distance));
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
            let sky = sky.chunk_sky(ChunkPos(pos));
            bytes += chunk.byte_size() + sky.byte_size() + MESSAGE_OVERHEAD;
            chunk_sender.send::<WorldChannel>(ChunkData {
                pos: ChunkPos(pos),
                chunk,
                sky,
            });
            view.sent.insert(pos);
        }
        view.complete = complete;
    }
}

/// Clients' render distance settings: capped at VIEW_DISTANCE sideways (at least
/// 2 chunks), the same up and down as always. Changing it re-centres the view,
/// so chunks now out of range are unloaded and nearer ones sent.
/// Runs in Update: lightyear clears received messages every frame
pub fn receive_view_distance(
    mut clients: Query<(&mut ChunkView, &mut MessageReceiver<SetViewDistance>), With<Connected>>,
) {
    for (mut view, mut receiver) in &mut clients {
        for SetViewDistance { chunks } in receiver.receive() {
            let sideways = (chunks as i32).clamp(2, VIEW_DISTANCE.x);
            let distance = IVec3::new(sideways, VIEW_DISTANCE.y, sideways);
            if view.distance != Some(distance) {
                view.distance = Some(distance);
                // makes `stream_chunks` unload what's now out of range
                view.center = None;
            }
        }
    }
}
