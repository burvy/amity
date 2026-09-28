//! Physics setup shared by client and server: avian, and its lightyear bridge.
//!
//! The bridge (`LightyearAvianPlugin`) also sets up the networking of physics:
//! on every rigid body (players, props) it replicates `Position`, `Rotation`
//! and both velocities, predicts them on the client, rolls back when the
//! server's differ by more than a hundredth, and smooths what's drawn: between
//! fixed ticks (frame interpolation) and after a rollback's correction, with a
//! curve that follows the velocities. See lightyear_avian3d's plugin.rs.

use avian3d::{
    dynamics::solver::islands::IslandSleepingPlugin, interpolation::PhysicsInterpolationPlugin,
    physics_transform::PhysicsTransformPlugin, PhysicsPlugins,
};
use bevy::prelude::*;
use lightyear::avian3d::plugin::LightyearAvianPlugin;

pub struct PhysicsProtocolPlugin;

impl Plugin for PhysicsProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default()
                .build()
                // sleeping bodies don't survive rollback
                .disable::<IslandSleepingPlugin>()
                // lightyear smooths what's drawn, and copies physics to
                // `Transform`, itself
                .disable::<PhysicsInterpolationPlugin>()
                .disable::<PhysicsTransformPlugin>(),
            // MUST be added manually, after the networking plugins
            LightyearAvianPlugin::default(),
        ));
    }
}
