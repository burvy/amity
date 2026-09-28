//! Player body and movement numbers, shared so client and server simulate
//! players identically. Speeds and the jump aim for how a real person moves

/// Walking speed (metres per second). A person walks at about 1.4
pub const WALK_SPEED: f32 = 4.0;
/// Running speed while sprint is held (metres per second)
pub const RUN_SPEED: f32 = 12.0;

/// How fast a player on the ground speeds up to, or slows down from, the speed
/// they want (metres per second, per second). At 20, running starts or stops
/// in a quarter of a second
pub const GROUND_ACCEL: f32 = 10.0;
/// How much a player can steer mid-air (metres per second, per second). Low,
/// so a jump mostly carries the speed it started with
pub const AIR_ACCEL: f32 = 2.0;

/// How high a jump lifts the body (metres). Clears one block (0.5 m); a person
/// jumps about half a metre from standing
pub const JUMP_HEIGHT: f32 = 1.25;
/// The fastest a falling player gets (metres per second): a person falling
/// through air stops speeding up at about this
pub const TERMINAL_SPEED: f32 = 50.0;

/// The body is an upright box: this wide and deep (metres)
pub const PLAYER_WIDTH: f32 = 0.6;
/// The body box's height (metres)
pub const PLAYER_HEIGHT: f32 = 1.8;
/// Distance from the body's centre to its top or bottom
pub const PLAYER_HALF_HEIGHT: f32 = PLAYER_HEIGHT / 2.0;

/// Eye height above the body's centre, for the camera and aiming. 1.6 m up
/// from the feet
pub const EYE_HEIGHT: f32 = 0.7;

/// How far below the body's bottom counts as standing on something (metres).
/// A little more than the gap move-and-slide keeps from surfaces
pub const GROUND_PROBE: f32 = 0.03;
/// A surface counts as ground if its normal points up at least this much
/// (cosine of its tilt: 0.7 is about 45 degrees). Block tops are 1, walls 0
pub const GROUND_NORMAL_MIN: f32 = 0.7;

/// Distance to consider interaction
pub const INTERACT_DISTANCE: f32 = 4.0;
