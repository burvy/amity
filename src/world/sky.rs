//! Keeping each chunk's view of the sky (`ChunkSky`, protocol `chunk/sky.rs`)
//! up to date as edits raise and lower columns. Sky light itself is worked out
//! when a chunk is meshed (`meshing/light.rs`)

use amity_protocol::world::{
    chunk::{ChunkIndex, ChunkPos, ChunkSky, ColumnSky},
    config::CHUNK_SIZE,
};
use bevy::{ecs::system::SystemParam, prelude::*};

/// A chunk's sky changed after it arrived: its light, and the light of the
/// chunks around it (light spreads across borders), has to be redone
#[derive(Message, Clone, Copy, Debug)]
pub struct SkyChanged(pub IVec3);

/// Changes the loaded chunks' view of the sky
#[derive(SystemParam)]
pub struct Skies<'w, 's> {
    index: Res<'w, ChunkIndex>,
    skies: Query<'w, 's, &'static mut ChunkSky>,
    changed: MessageWriter<'w, SkyChanged>,
}

impl Skies<'_, '_> {
    /// The column holding the block at `pos` changed height. Every chunk of the
    /// column between the old and new height sees the sky differently there
    pub fn column_changed(&mut self, pos: IVec3, change: ColumnSky) {
        let (low, high) = (change.was.min(change.now), change.was.max(change.now));
        let local = pos.rem_euclid(IVec3::splat(CHUNK_SIZE));
        for y in low.div_euclid(CHUNK_SIZE)..=high.div_euclid(CHUNK_SIZE) {
            let chunk = ChunkPos(IVec3::new(
                pos.x.div_euclid(CHUNK_SIZE),
                y,
                pos.z.div_euclid(CHUNK_SIZE),
            ));
            let Some(mut sky) = self.index.get(chunk).and_then(|e| self.skies.get_mut(e).ok())
            else {
                continue;
            };
            if sky.set_height(y, local.x, local.z, change.now) {
                self.changed.write(SkyChanged(chunk.0));
            }
        }
    }
}
