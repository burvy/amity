use bevy::math::Vec3;

/// Mouse look speed at a sensitivity setting of 1.0: radians turned per pixel
/// the mouse moves. The player's sensitivity setting multiplies it
pub const BASE_SENSITIVITY: f32 = 0.0025;

/// Seconds between two jump presses that count as a double-tap (to fly)
pub const DOUBLE_TAP_SECS: f32 = 0.3;

/// After a step up or down, the camera and model glide the rest of the way at
/// the player's walking speed (a 45 degree slope: blocks are as tall as they
/// are deep), so a staircase feels like a ramp. At least this fast (metres per
/// second), so a step taken nearly standing still doesn't hang
pub const STEP_GLIDE_MIN_SPEED: f32 = 2.0;

// Key bindings are the player's settings now: see `settings/definition.rs`
// (`Action::default_binding` for the defaults)

/// The flashlight comes on by itself when the sun sinks below this height
/// (degrees above the horizon: negative is below it), and goes off when it
/// rises back above
pub const FLASHLIGHT_NIGHT_SUN_DEGREES: f32 = -2.0;
/// The flashlight's brightness (lumens, counted over a whole sphere, as bevy's
/// spot lights are). Far more than a real one: the camera's exposure is fixed
/// for daylight (`sky/config.rs`), so a real flashlight would barely show
pub const FLASHLIGHT_LUMENS: f32 = 1.0e7;
/// How far the beam reaches (metres)
pub const FLASHLIGHT_RANGE: f32 = 40.0;
/// The beam's edge and its bright middle, as angles from its centre (radians)
pub const FLASHLIGHT_OUTER_ANGLE: f32 = 0.35;
pub const FLASHLIGHT_INNER_ANGLE: f32 = 0.2;
/// Where the flashlight is held, from the eye (right, up, forward in metres):
/// a little off to the side, so its shadows aren't hidden behind what they fall
/// from
pub const FLASHLIGHT_HAND: Vec3 = Vec3::new(0.25, -0.2, 0.0);
