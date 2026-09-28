//! The sky and its light: the sun and moon where astronomy puts them, lighting
//! the world through a physically simulated atmosphere, seen through a camera
//! that adapts to brightness like eyes do.
//!
//! - `config.rs`: light levels in real units, shadows, exposure
//! - `logic.rs`: the lights, atmosphere and camera, and keeping time
//!
//! The time and the astronomy are shared (protocol `sky`); the server owns the
//! clock.

use bevy::{post_process::auto_exposure::AutoExposurePlugin, prelude::*};

mod config;
pub(crate) mod logic;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        // not part of the default plugins
        app.add_plugins(AutoExposurePlugin);
        app.init_resource::<logic::SkyTime>();
        app.add_systems(Startup, logic::spawn_sky);
        app.add_observer(logic::setup_camera);
        app.add_systems(Update, (logic::update_time, logic::move_lights).chain());
    }
}
