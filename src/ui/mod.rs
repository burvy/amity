//! On-screen UI: the hotbar and inventory, the creative block picker, the
//! debug screen, and the pause menu.
//!
//! - `crosshair.rs`: the crosshair, spreading with speed
//! - `config.rs`: sizes and colours (placeholder styling, meant to be redesigned)
//! - `font.rs`: the one font all UI text uses (swap in your own there)
//! - `debug.rs`: the debug screen (F3)
//! - `pause.rs`: the pause menu (Esc)
//! - `help.rs`: the Help page, from `assets/help.md`
//! - `settings.rs`: the Settings page (Graphics and Controls tabs)
//! - `scroll.rs`: scrolling for the pause menu's pages
//! - `inventory.rs`: the hotbar down the right edge, the inventory that slides
//!   out beside it while Tab is held, and the item held on the cursor
//! - `picker.rs`: the creative block picker, beside the open inventory

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};

use crate::world::meshing::material::BlockAtlas;

mod config;
mod crosshair;
mod debug;
pub mod font;
mod help;
mod inventory;
mod pause;
mod picker;
mod scroll;
mod settings;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // the debug screen's frame rate
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        // the debug screen's GPU time per render pass (native: the browser
        // gives no GPU timings)
        #[cfg(not(target_family = "wasm"))]
        app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
        app.init_resource::<font::UiFont>();
        app.init_resource::<pause::PausePage>();
        app.init_resource::<settings::SettingsTab>();
        app.init_resource::<settings::Rebinding>();
        app.init_resource::<crosshair::Spread>();
        app.add_systems(
            Startup,
            (
                debug::spawn_debug,
                pause::spawn_pause_menu,
                crosshair::spawn_crosshair,
            ),
        );
        app.add_systems(
            Update,
            (
                crosshair::show_crosshair,
                (crosshair::spread_crosshair, crosshair::place_crosshair).chain(),
            ),
        );
        app.add_systems(
            Update,
            (
                pause::escape,
                // before the clicks: a click that sets a binding is used up
                settings::rebind,
                pause::click_menu,
                settings::click_settings,
                pause::show_menu,
                settings::update_settings,
                scroll::scroll,
            )
                .chain(),
        );
        #[cfg(target_family = "wasm")]
        app.add_systems(Update, pause::pause_when_mouse_released);
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
