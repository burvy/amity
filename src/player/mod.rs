//! Our player and the ones we see: input, camera, bodies, and the interact prompt

use amity_protocol::player::{movement, step};
use avian3d::prelude::Position;
use bevy::{prelude::*, transform::TransformSystems};
use lightyear::{
    frame_interpolation::FrameInterpolationPlugin,
    input::client::InputSystems,
    prelude::{Predicted, RollbackSystems},
};

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

        app.add_systems(
            Update,
            (
                input::select_slot,
                input::toggle_flight,
                input::rotate_player_resource,
                interact::client_interaction_checker,
                interact::can_interact_prompt,
                // players replicated to us get a body to simulate and a mesh to see
                logic::add_physics,
                logic::draw_players,
            ),
        );

        // Smooth players between fixed ticks. The camera follows the smoothed
        // position, so it goes after interpolation (and rollback's visual
        // correction), and before transforms are handed to rendering
        app.add_plugins(FrameInterpolationPlugin::<Position>::default());
        app.add_systems(
            PostUpdate,
            logic::camera_follow_player
                .after(RollbackSystems::VisualCorrection)
                .before(TransformSystems::Propagate),
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
