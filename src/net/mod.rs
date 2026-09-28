//! Allow the client to directly connect to the server, receive replicated
//! information, and predict the world client side for instant response rather
//! than waiting for the server.

use std::time::Duration;

use amity_protocol::net::config::TIMESTEP;
use bevy::prelude::*;
use lightyear::prelude::client::ClientPlugins;

mod config;
mod logic;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ClientPlugins {
            tick_duration: Duration::from_secs_f64(TIMESTEP),
        });
        let (prediction, input_timeline) = logic::prediction_resources();
        app.insert_resource(prediction);
        app.insert_resource(input_timeline);
        app.add_systems(Startup, logic::connect);
    }
}
