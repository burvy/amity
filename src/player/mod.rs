use bevy::prelude::*;

pub mod definition;
pub mod logic;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // This only spawned a camera and we spawn a camera in `net.rs`
        // app.add_systems(Startup, definition::player.spawn());

        app.insert_resource(definition::PlayerInfo {
            look: Vec2::ZERO,
            paused: true,
            can_interact: false,
        });

        app.add_systems(
            Startup,
            (definition::spawn_camera, definition::spawn_interact_prompt),
        );
        app.add_systems(
            Update,
            (
                logic::grab_on_click,
                logic::pause_on_esc,
                logic::rotate_player_resource,
                logic::camera_follow_player,
                logic::client_interaction_checker,
                logic::can_interact_prompt,
            ),
        );
    }
}
