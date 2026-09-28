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
