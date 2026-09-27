use bevy::prelude::*;

mod net;
mod player;
mod sky;
mod stuff;
mod ui;
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
                canvas: Some("#amity-canvas".into()),
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
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(sky::SkyPlugin);
        app.add_plugins(stuff::StuffPlugin);
        app.add_plugins(ui::UiPlugin);
        app.add_plugins(world::WorldPlugin);
        app.add_plugins(net::NetPlugin);
        // brings physics with it, so the client can actually simulate
        app.add_plugins(amity_protocol::ProtocolPlugin);

        // dev-local only. EguiPlugin must come first - the inspector asserts
        // on it at startup rather than pulling it in itself.
        // Hidden until F3: it walks every entity each frame, and every chunk is one
        #[cfg(feature = "dev-local")]
        app.add_plugins((
            bevy_inspector_egui::bevy_egui::EguiPlugin::default(),
            bevy_inspector_egui::quick::WorldInspectorPlugin::new().run_if(
                bevy::input::common_conditions::input_toggle_active(false, KeyCode::F3),
            ),
        ));
    }
}
