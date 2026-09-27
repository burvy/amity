//! Our player and the ones we see: input, camera, bodies, and the interact prompt

use amity_protocol::player::{movement, step};
use bevy::prelude::*;
use lightyear::{input::client::InputSystems, prelude::Predicted};

pub(crate) mod config;
pub mod definition;
mod input;
mod interact;
mod logic;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(definition::PlayerInfo {
            paused: true,
            ..default()
        });

        app.add_systems(
            Startup,
            (definition::spawn_camera, definition::spawn_interact_prompt),
        );

        let grab_on_click = input::grab_on_click;
        // stop clicking egui from unpausing the game
        // TODO: possibly better to make a new window
        #[cfg(feature = "dev-local")]
        let grab_on_click = grab_on_click.run_if(
            |wants: Res<bevy_inspector_egui::bevy_egui::input::EguiWantsInput>| {
                !wants.wants_pointer_input()
            },
        );

        app.add_systems(
            Update,
            (
                grab_on_click,
                input::pause_on_esc,
                input::select_slot,
                input::toggle_flight,
                (input::rotate_player_resource, logic::camera_follow_player).chain(),
                interact::client_interaction_checker,
                interact::can_interact_prompt,
                // players replicated to us get a body to simulate and a mesh to see
                logic::add_physics,
                logic::draw_players,
            ),
        );

        app.add_observer(logic::detect_replicate_player);
        app.add_observer(logic::detect_replicate_our_player);

        // Client-server interface (tick rate update)
        app.add_systems(
            FixedPreUpdate,
            input::buffer_input.in_set(InputSystems::WriteClientInputs),
        );

        // Simulate players
        app.add_systems(
            FixedUpdate,
            (
                movement::player_motion::<With<Predicted>>,
                step::step_up::<With<Predicted>>,
            )
                .chain(),
        );
    }
}
