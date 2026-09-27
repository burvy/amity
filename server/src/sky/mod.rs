//! The world clock: advances with the server's ticks and is replicated to
//! clients about once a second

use bevy::prelude::*;

mod logic;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<logic::Clock>();
        app.add_systems(Startup, logic::spawn_clock);
        app.add_systems(FixedUpdate, logic::advance_clock);
    }
}
