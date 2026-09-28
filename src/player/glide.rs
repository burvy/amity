//! Smoothing steps: movement lifts a player onto a ledge (or down off one) in a
//! single tick, which is right for the simulation but looks like a jolt. Here
//! the camera and the player's model lag behind that jump and glide the rest of
//! the way, at the player's walking speed, so a staircase looks and feels like
//! a ramp. Purely visual: the body itself stays exactly where movement put it.
//!
//! How: every step is added to a running `total`. The drawn position is frame
//! interpolated, so it blends from the last tick's total to this tick's too.
//! `shown` follows `total` at a limited speed; the gap between `shown` and the
//! blended total is how far off the drawn position the camera and model sit.

use amity_protocol::player::movement::Stepped;
use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;

use super::config::STEP_GLIDE_MIN_SPEED;

/// A player's step smoothing (see the module docs). Heights in metres
#[derive(Component, Default, Debug)]
pub struct StepGlide {
    /// All the steps so far, as of the latest tick
    total: f32,
    /// `total` as of the tick before
    previous: f32,
    /// Where the camera and model have glided to
    shown: f32,
    /// This frame's gap: how far to draw the camera and model from the body
    pub offset: f32,
}

/// Adds each tick's step to the running total. Runs after every tick's
/// movement, but not while lightyear replays ticks to correct a prediction:
/// those replays redo steps already counted
pub fn record_steps(mut players: Query<(&Stepped, &mut StepGlide)>) {
    for (stepped, mut glide) in &mut players {
        glide.previous = glide.total;
        glide.total += stepped.0;
    }
}

/// Glides `shown` toward the total and works out this frame's offset. Runs
/// after frame interpolation, before the camera and models are placed
pub fn glide_steps(
    mut players: Query<(&LinearVelocity, &mut StepGlide)>,
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
) {
    for (velocity, mut glide) in &mut players {
        let speed = velocity.0.with_y(0.0).length().max(STEP_GLIDE_MIN_SPEED);
        let gap = glide.total - glide.shown;
        let reach = speed * time.delta_secs();
        glide.shown = if gap.abs() <= reach {
            glide.total
        } else {
            glide.shown + reach * gap.signum()
        };
        let drawn = glide.previous.lerp(glide.total, fixed.overstep_fraction());
        glide.offset = glide.shown - drawn;

        // caught up: start the running total over, so it never grows large
        if glide.shown == glide.total && glide.previous == glide.total {
            *glide = StepGlide::default();
        }
    }
}
