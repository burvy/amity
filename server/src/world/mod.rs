//! The server's voxel world: generating it, streaming it to clients, and
//! applying their edits

use bevy::prelude::*;

mod area;
mod edit;
pub mod generation;
pub mod streaming;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((generation::GenerationPlugin, streaming::StreamingPlugin));
        // Update, not FixedUpdate: lightyear clears received messages every frame,
        // and a frame with no fixed tick would drop them
        app.add_systems(Update, edit::receive_edits);
    }
}
