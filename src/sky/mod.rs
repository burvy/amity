//! The sky and its light: the sun and moon where astronomy puts them, lighting
//! the world through a physically simulated atmosphere, seen through a camera
//! with a fixed exposure.
//!
//! - `config.rs`: light levels in real units, shadows, exposure
//! - `logic.rs`: the lights, atmosphere and camera, and keeping time
//!
//! The time and the astronomy are shared (protocol `sky`); the server owns the
//! clock.

use bevy::prelude::*;

pub(crate) mod config;
pub(crate) mod logic;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<logic::SkyTime>();
        app.add_systems(Startup, logic::spawn_sky);
        app.add_observer(logic::setup_camera);
        app.add_systems(Update, (logic::update_time, logic::move_lights).chain());
    }
}
