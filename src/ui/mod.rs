//! On-screen UI: the hotbar, the creative block picker, and the debug screen.
//!
//! - `config.rs`: sizes and colours (placeholder styling, meant to be redesigned)
//! - `font.rs`: the one font all UI text uses (swap in your own there)
//! - `debug.rs`: the debug screen (F3)
//! - `inventory.rs`: the hotbar down the right edge, the inventory that slides
//!   out beside it while Tab is held, and the item held on the cursor
//! - `picker.rs`: the creative block picker, beside the open inventory

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};

use crate::world::meshing::material::BlockAtlas;

mod config;
mod debug;
pub mod font;
mod inventory;
mod picker;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // the debug screen's frame rate
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        app.init_resource::<font::UiFont>();
        app.add_systems(Startup, debug::spawn_debug);
        app.add_systems(Update, (debug::toggle_debug, debug::update_debug).chain());
        app.add_systems(
            Update,
            (
                // built once the block atlas (for icons) exists
                (inventory::spawn_inventory, picker::spawn_picker)
                    .run_if(resource_added::<BlockAtlas>),
                inventory::open_inventory,
                inventory::slide_panel,
                inventory::click_slots,
                inventory::update_slots,
                picker::show_picker,
                picker::click_picker,
            )
                .chain(),
        );
    }
}
