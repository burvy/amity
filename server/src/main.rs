// bevy query types are inherently verbose; a type alias for a single-use
// query reads worse than the query. bevy's own crates allow this too.
#![allow(clippy::type_complexity)]

use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*, state::app::StatesPlugin};
use shooter_protocol::protocol;
mod generation;
mod logic;
mod server;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            protocol::TIMESTEP / 4.0, // run more often than timestep
        ))),
        TransformPlugin,
        StatesPlugin,
        LogPlugin::default(),
    ));
    app.add_plugins(server::ServerPlugin);
    app.add_plugins(protocol::ProtocolPlugin);
    app.run();
}
