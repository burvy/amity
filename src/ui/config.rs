//! How the UI looks: sizes in pixels, and colours. Placeholder styling, meant to
//! be redesigned

use bevy::prelude::*;

/// Hotbar: one square slot per item, stacked on the right edge of the screen.
/// Each slot shows `assets/images/ui/hotbar-icon.png`, and the selected one has
/// `hotbar-outline.png` on top. Both are 56 x 56: at this size they're drawn 1:1
pub const HOTBAR_SLOT_SIZE: f32 = 56.0;
/// The item icon inside a slot
pub const HOTBAR_ICON_SIZE: f32 = 40.0;
/// Space between slots
pub const HOTBAR_GAP: f32 = 6.0;
/// Space between the hotbar and the right edge
pub const HOTBAR_MARGIN: f32 = 16.0;
/// The item count, in a slot's bottom-right corner
pub const HOTBAR_COUNT_SIZE: f32 = 14.0;

/// Block picker (creative): a grid of every item over a dimmed screen
pub const PICKER_COLUMNS: u16 = 9;
pub const PICKER_SLOT_SIZE: f32 = 52.0;
pub const PICKER_ICON_SIZE: f32 = 40.0;
pub const PICKER_GAP: f32 = 6.0;
pub const PICKER_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
pub const PICKER_SLOT: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);
pub const PICKER_SLOT_HOVERED: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);

/// Debug screen (F3): text in the top-left corner on a dark box
pub const DEBUG_FONT_SIZE: f32 = 14.0;
pub const DEBUG_MARGIN: f32 = 8.0;
pub const DEBUG_PADDING: f32 = 6.0;
pub const DEBUG_BACKGROUND: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
