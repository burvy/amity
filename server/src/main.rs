// bevy query types are inherently verbose; a type alias for a single-use
// query reads worse than the query. bevy's own crates allow this too.
#![allow(clippy::type_complexity)]

use std::time::Duration;

use amity_protocol::{net::config::TIMESTEP, ProtocolPlugin};
use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*, state::app::StatesPlugin};

mod connection;
mod player;
mod sky;
mod stuff;
mod world;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            TIMESTEP / 4.0, // run more often than timestep
        ))),
        TransformPlugin,
        StatesPlugin,
        LogPlugin::default(),
    ));
    // connection first: it adds lightyear's server plugins, which the rest build on
    app.add_plugins((
        connection::ConnectionPlugin,
        player::PlayerPlugin,
        sky::SkyPlugin,
        stuff::StuffPlugin,
        world::WorldPlugin,
    ));
    app.add_plugins(ProtocolPlugin);
    app.run();
}
