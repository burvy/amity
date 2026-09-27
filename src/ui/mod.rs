//! On-screen UI: the hotbar and the creative block picker.
//!
//! - `config.rs`: sizes and colours (placeholder styling, meant to be redesigned)
//! - `hotbar.rs`: the hotbar down the right edge
//! - `picker.rs`: the creative block picker (Tab)

use bevy::prelude::*;

use crate::world::meshing::material::BlockAtlas;

mod config;
mod hotbar;
mod picker;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                // built once the block atlas (for icons) exists
                hotbar::spawn_hotbar.run_if(resource_added::<BlockAtlas>),
                hotbar::update_hotbar,
                picker::toggle_picker,
                picker::click_picker,
            ),
        );
    }
}
