//! The player's settings: graphics and controls, kept between sessions.
//!
//! - `definition.rs`: every setting, its default, and the key bindings
//! - `storage.rs`: saving and loading (a file natively, browser storage on the web)
//! - `input.rs`: `ActionInput`, reading keys through the bindings
//! - `apply.rs`: putting graphics settings into effect
//!
//! The screens to change them are in `ui/settings.rs`.

use bevy::prelude::*;

pub mod apply;
pub mod definition;
pub mod input;
mod storage;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        // loaded before anything reads them
        app.insert_resource(storage::load());
        app.init_resource::<storage::SaveTimer>();
        app.add_systems(
            Update,
            (
                apply::apply_graphics.run_if(resource_changed::<definition::Settings>),
                apply::send_render_distance,
                storage::save_when_changed,
            ),
        );
        #[cfg(not(target_family = "wasm"))]
        app.add_systems(Last, apply::limit_frame_rate);
    }
}
