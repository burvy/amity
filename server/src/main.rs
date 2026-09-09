use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*, state::app::StatesPlugin};
use burvy_protocol::protocol;
mod server;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            protocol::TIMESTEP / 4.0, // run more often than timestep
        ))),
        TransformPlugin::default(),
        StatesPlugin::default(),
        LogPlugin::default(),
    ));
    app.add_plugins(server::ServerPlugin);
    app.add_plugins(protocol::ProtocolPlugin);
    app.run();
}
