//! Sends each client the chunks around its player, nearest first, within a
//! bandwidth budget, and tells it to drop chunks it has walked away from

use bevy::prelude::*;

pub mod config;
pub mod logic;

pub struct StreamingPlugin;

impl Plugin for StreamingPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(logic::add_chunk_view);
        app.add_systems(FixedUpdate, logic::stream_chunks);
        app.add_systems(Update, logic::receive_view_distance);
    }
}
