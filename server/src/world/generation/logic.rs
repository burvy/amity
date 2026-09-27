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
    terrain::Terrain,
};
use crate::{
    player::config::SPAWN_XZ,
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
    terrain: Res<Terrain>,
    mut generating: ResMut<Generating>,
    index: Res<ChunkIndex>,
    players: Query<&Position, With<PlayerMarker>>,
    mut offsets: Local<Vec<IVec3>>,
    mut done: Local<HashSet<IVec3>>,
) {
    // The box of chunk offsets to generate around each anchor, sorted nearest
    // first. Built once on the first run (`Local` keeps it between runs): the view
    // distance, plus the pregeneration margin on top
    if offsets.is_empty() {
        *offsets = area::nearest_first(VIEW_DISTANCE + PREGEN_MARGIN);
    }
    let pool = AsyncComputeTaskPool::get();

    // Anchors: the chunks to generate around. Spawn always comes first, so the
    // ground there exists before anyone joins; then each player's chunk
    let anchors = iter::once(terrain.surface_point(SPAWN_XZ))
        .chain(players.iter().map(|position| position.0))
        .map(|point| ChunkPos::containing(point).0);

    for anchor in anchors {
        // everything around this chunk already exists (see `done` above)
        if done.contains(&anchor) {
            continue;
        }
        // stays true only if every chunk in the box already exists
        let mut complete = true;
        // walk the box nearest first, so the closest missing chunks start first
        for offset in offsets.iter() {
            let pos = anchor + *offset;
            // already generated and spawned
            if index.contains(ChunkPos(pos)) {
                continue;
            }
            complete = false;
            // already being generated
            if generating.0.contains_key(&pos) {
                continue;
            }
            // enough running at once: stop here, the rest start on later ticks
            // as these finish. Nearest-first order means nothing closer is skipped
            if generating.0.len() >= MAX_GENERATING {
                return;
            }
            // `pool.spawn` runs the closure on a background thread and returns a
            // `Task`, a handle to its eventual result, checked in `finish_generating`.
            // each task gets its own handle: tasks can't borrow resources
            let terrain = terrain.clone();
            let task = pool.spawn(async move { terrain.generate(ChunkPos(pos)) });
            generating.0.insert(pos, task);
        }
        if complete {
            done.insert(anchor);
        }
    }
}

/// Spawns the chunks whose generation finished
/// Runs right after `start_generating` each tick. A chunk is only spawned after
/// its task is done, so a half-built chunk is never visible
pub fn finish_generating(mut cmds: Commands, mut generating: ResMut<Generating>) {
    // `retain` keeps the entries the closure returns true for. `check_ready` peeks
    // at a task without waiting: Some(chunk) once finished, None while running.
    // Finished tasks spawn their chunk (streaming then sends it to clients) and
    // are dropped from the map; running ones stay for the next tick
    generating.0.retain(|&pos, task| match check_ready(task) {
        Some(chunk) => {
            cmds.spawn(chunk_body(ChunkPos(pos), chunk));
            false
        }
        None => true,
    });
}
