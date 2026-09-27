//! Starting the server and accepting clients

use std::time::Duration;

use amity_protocol::net::config::TIMESTEP;
use bevy::prelude::*;
use lightyear::prelude::server::ServerPlugins;

mod config;
mod logic;

pub struct ConnectionPlugin;

impl Plugin for ConnectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(TIMESTEP),
        });
        app.add_systems(Startup, logic::start_server);
        app.add_systems(Update, logic::heartbeat);
    }
}
