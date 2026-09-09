use bevy::prelude::*;

mod net;
mod player;
mod world;

// This section below allows the game to render on the web
// ---
use std::sync::atomic::{AtomicBool, Ordering};

static STARTED: AtomicBool = AtomicBool::new(false);

pub fn run() {
    if STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                canvas: Some("#game-canvas".into()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MainPlugin)
        .run();
}
// ---

pub struct MainPlugin;

impl Plugin for MainPlugin {
    fn build(&self, app: &mut App) {
        // brings physics with it, so the client can actually simulate
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(world::WorldPlugin);
        app.add_plugins(net::NetPlugin);
        app.add_plugins(shooter_protocol::protocol::ProtocolPlugin);
    }
}
