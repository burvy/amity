use std::iter;

use amity_protocol::{
    player::definition::PlayerMarker,
    world::chunk::{chunk_body, Chunk, ChunkIndex, ChunkPos},
};
use avian3d::prelude::Position;
use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};

use super::{
    config::{MAX_GENERATING, PREGEN_MARGIN},
    settings::WorldGenSettings,
    terrain,
};
use crate::{
    player::config::SPAWN_POINT,
    world::{area, streaming::config::VIEW_DISTANCE},
};

/// Chunks being generated on background threads, by position
#[derive(Resource, Default)]
pub struct Generating(HashMap<IVec3, Task<Chunk>>);

/// Starts generating the missing chunks around spawn and every player, nearest
/// first, keeping at most MAX_GENERATING in flight.
/// Chunks are never unloaded on the server (edits would be lost until there are
/// save files), so an area that was once complete stays complete; `done`
/// remembers those so they aren't rechecked every tick.
pub fn start_generating(
    settings: Res<WorldGenSettings>,
    mut generating: ResMut<Generating>,
    index: Res<ChunkIndex>,
    players: Query<&Position, With<PlayerMarker>>,
    mut offsets: Local<Vec<IVec3>>,
    mut done: Local<HashSet<IVec3>>,
) {
    if offsets.is_empty() {
        *offsets = area::nearest_first(VIEW_DISTANCE + PREGEN_MARGIN);
    }
    let pool = AsyncComputeTaskPool::get();
    let anchors = iter::once(SPAWN_POINT)
        .chain(players.iter().map(|position| position.0))
        .map(|point| ChunkPos::containing(point).0);

    for anchor in anchors {
        if done.contains(&anchor) {
            continue;
        }
        let mut complete = true;
        for offset in offsets.iter() {
            let pos = anchor + *offset;
            if index.contains(ChunkPos(pos)) {
                continue;
            }
            complete = false;
            if generating.0.contains_key(&pos) {
                continue;
            }
            if generating.0.len() >= MAX_GENERATING {
                return;
            }
            // each task gets its own copy of the settings: tasks can't borrow resources
            let settings = settings.clone();
            let task = pool.spawn(async move { terrain::generate(&settings, ChunkPos(pos)) });
            generating.0.insert(pos, task);
        }
        if complete {
            done.insert(anchor);
        }
    }
}

/// Spawns the chunks whose generation finished
pub fn finish_generating(mut cmds: Commands, mut generating: ResMut<Generating>) {
    generating.0.retain(|&pos, task| match check_ready(task) {
        Some(chunk) => {
            cmds.spawn(chunk_body(ChunkPos(pos), chunk));
            false
        }
        None => true,
    });
}
