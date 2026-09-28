//! Player body and movement numbers, shared so client and server simulate
//! players identically

use crate::world::config::BLOCK_SIZE;

/// Walking speed (metres per second)
pub const WALK_SPEED: f32 = 1.5;
/// Running speed while sprint is held (metres per second)
pub const RUN_SPEED: f32 = 8.0;

/// How fast a player on the ground speeds up to, or slows down from, the speed
/// they want (metres per second, per second)
pub const GROUND_ACCEL: f32 = 10.0;
/// How much a player can steer mid-air (metres per second, per second)
pub const AIR_ACCEL: f32 = 2.0;

/// How high a jump lifts the body (metres)
pub const JUMP_HEIGHT: f32 = 1.1;
/// The fastest a falling player gets (metres per second)
pub const TERMINAL_SPEED: f32 = 50.0;

/// The body is an upright box: this wide and deep (metres)
pub const PLAYER_WIDTH: f32 = 0.6;
/// The body box's height (metres)
pub const PLAYER_HEIGHT: f32 = 1.8;
/// Distance from the body's centre to its top or bottom
pub const PLAYER_HALF_HEIGHT: f32 = PLAYER_HEIGHT / 2.0;

/// Eye height above the body's centre, for the camera and aiming
pub const EYE_HEIGHT: f32 = 0.7;

/// How far below the body's bottom counts as standing on something (metres).
/// A little more than the gap move-and-slide keeps from surfaces
pub const GROUND_PROBE: f32 = 0.03;
/// A surface counts as ground if its normal points up at least this much
/// (cosine of its tilt). Block tops are 1, walls 0
pub const GROUND_NORMAL_MIN: f32 = 0.7;

/// The highest ledge a walking player goes up onto without jumping, and the
/// deepest drop they stay on the ground over rather than falling: one block,
/// plus a margin
pub const STEP_HEIGHT: f32 = BLOCK_SIZE + 0.05;

/// Distance to consider interaction
pub const INTERACT_DISTANCE: f32 = 4.0;
