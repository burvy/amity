//! Everything the client and server must agree on: network settings, replicated
//! components, messages, and the simulation code both sides run identically.
//! Each folder is one feature; its `mod.rs` is the plugin that registers it.

use bevy::prelude::*;

pub mod inventory;
pub mod net;
pub mod physics;
pub mod player;
pub mod sky;
pub mod stuff;
pub mod world;

/// Registers everything shared. Added by both the client and the server
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            player::PlayerProtocolPlugin,
            inventory::InventoryProtocolPlugin,
            sky::SkyProtocolPlugin,
            stuff::StuffProtocolPlugin,
            world::WorldProtocolPlugin,
            physics::PhysicsProtocolPlugin,
        ));
    }
}
