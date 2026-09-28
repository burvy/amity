//! Horizon shadows: the shadows of hills and mountains, far beyond where the
//! shadow map reaches, for next to nothing.
//!
//! For every 2 m cell of loaded ground, the map stores how high the terrain
//! rises around it in 8 directions (like standing there and noting the angle
//! of the skyline all round). When the sun is lower than the skyline in its
//! direction, the cell is in shadow. That's worked out once, on a background
//! thread, and only again as chunks change or the player moves far; drawing
//! it is a texture read per pixel. It follows the sun (or moon) exactly as it
//! moves, which baked shadows couldn't.
//!
//! Near the player, the shadow map still draws shadows exactly, of everything;
//! the horizon adds the big shapes' shadows everywhere, even with Shadows Off.
//! The block shader (`meshing/block.wgsl`) applies it, and also keeps sunlight
//! out of places the sky can't reach, like caves.
//!
//! - `config.rs`: cell size, map size, how far to look, softness
//! - `compute.rs`: working out the map (background thread)
//! - `logic.rs`: when to redo it, and putting it in the texture

use bevy::prelude::*;

pub mod compute;
pub mod config;
pub mod logic;

pub struct HorizonPlugin;

impl Plugin for HorizonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<logic::HorizonUpdate>();
        app.add_systems(PreStartup, logic::init_horizon_map);
        app.add_systems(Update, (logic::start_update, logic::finish_update).chain());
    }
}
