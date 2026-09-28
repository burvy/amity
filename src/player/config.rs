/// Mouse look speed at a sensitivity setting of 1.0: radians turned per pixel
/// the mouse moves. The player's sensitivity setting multiplies it
pub const BASE_SENSITIVITY: f32 = 0.0025;

/// Seconds between two jump presses that count as a double-tap (to fly)
pub const DOUBLE_TAP_SECS: f32 = 0.3;

// Key bindings are the player's settings now: see `settings/definition.rs`
// (`Action::default_binding` for the defaults)
