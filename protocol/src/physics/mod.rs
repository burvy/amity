//! Physics setup shared by client and server: avian, its lightyear bridge, and
//! which physics components are replicated and predicted.

use avian3d::{
    dynamics::{rigid_body::LinearVelocity, solver::islands::IslandSleepingPlugin},
    interpolation::PhysicsInterpolationPlugin,
    physics_transform::{PhysicsTransformConfig, PhysicsTransformPlugin, Position},
    PhysicsPlugins,
};
use bevy::prelude::*;
use lightyear::{
    avian3d::plugin::LightyearAvianPlugin,
    interpolation::registry::InterpolationRegistrationExt,
    prediction::registry::PredictionBuilderExt,
    prelude::AppComponentExt,
};

pub mod config;
mod rollback;

pub struct PhysicsProtocolPlugin;

impl Plugin for PhysicsProtocolPlugin {
    fn build(&self, app: &mut App) {
        // ---
        // Having gravity set to 0 here would be confusing.
        // to let the player fall, unlock translation_y in player/definition.rs
        // ---

        app.component::<Position>()
            .replicate() // is sent over the link
            .predict() // is simulated by clients
            .with_rollback_condition(rollback::position)
            // when a rollback corrects a predicted player, blend the drawn
            // position over to the corrected one instead of snapping. Also what
            // hands the corrected position to frame interpolation after a
            // rollback: without it, frame interpolation restores the stale
            // pre-rollback position and the correction is lost for good
            .add_linear_correction_fn()
            .add_linear_interpolation(); // visual interpolation

        // linear velocity does not require interpolation like position
        // because interpolation is only needed for visual things,
        // velocity is basically invisible
        app.component::<LinearVelocity>()
            .replicate() // is sent over the link
            .predict() // is simulated by clients
            .with_rollback_condition(rollback::velocity);

        app.add_plugins((
            PhysicsPlugins::default()
                .build()
                .disable::<IslandSleepingPlugin>()
                .disable::<PhysicsInterpolationPlugin>()
                .disable::<PhysicsTransformPlugin>(),
            LightyearAvianPlugin::default(), // MUST be added manually
        ));
        // Nothing here is moved through Transform, so don't copy Transform into
        // Position. That copy runs after rollback and restored last frame's stale
        // Transform over the corrected Position, so predicted players that got
        // pushed out of place were never corrected (lightyear_avian3d plugin.rs TODO)
        app.insert_resource(PhysicsTransformConfig {
            transform_to_position: false,
            ..default()
        });
    }
}
