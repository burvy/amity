use bevy::prelude::*;

/// Mouse sensitivity constant
/// Can be kept client side for now since it should be configurable later
/// TODO: Make configurable
pub const SENSITIVITY: f32 = 0.0025;

// Key bindings, all in one place until there's a controls screen.
// Not Ctrl: Ctrl+W closes the browser tab on the web.
pub const KEY_FORWARD: KeyCode = KeyCode::KeyW;
pub const KEY_BACK: KeyCode = KeyCode::KeyS;
pub const KEY_LEFT: KeyCode = KeyCode::KeyA;
pub const KEY_RIGHT: KeyCode = KeyCode::KeyD;
/// Jump; while flying, up. Double-tap to start or stop flying (creative)
pub const KEY_JUMP: KeyCode = KeyCode::Space;
pub const KEY_SPRINT: [KeyCode; 2] = [KeyCode::ShiftLeft, KeyCode::ShiftRight];
/// Crouch; while flying, down
pub const KEY_CROUCH: KeyCode = KeyCode::KeyC;
pub const KEY_INTERACT: KeyCode = KeyCode::KeyE;
/// Opens the creative block picker
pub const KEY_PICKER: KeyCode = KeyCode::Tab;
/// Selects hotbar slots 1 to 5
pub const KEY_SLOTS: [KeyCode; 5] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
];
pub const BUTTON_PRIMARY: MouseButton = MouseButton::Left;
pub const BUTTON_SECONDARY: MouseButton = MouseButton::Right;

/// Seconds between two jump presses that count as a double-tap
pub const DOUBLE_TAP_SECS: f32 = 0.3;
