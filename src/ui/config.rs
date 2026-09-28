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

/// Inventory: 4 more columns of slots (same size and gap as the hotbar) that
/// slide out from the right edge, next to the hotbar, while Tab is held.
/// Seconds the slide takes (roughly: it eases in)
pub const INVENTORY_SLIDE_SECS: f32 = 0.12;
/// The item held on the mouse cursor: its icon size
pub const CURSOR_ICON_SIZE: f32 = 40.0;

/// Block picker (creative): a grid of every item, left of the inventory
pub const PICKER_COLUMNS: u16 = 6;
pub const PICKER_SLOT_SIZE: f32 = 52.0;
pub const PICKER_ICON_SIZE: f32 = 40.0;
pub const PICKER_GAP: f32 = 6.0;
/// Space between the picker and the inventory
pub const PICKER_MARGIN: f32 = 24.0;
pub const PICKER_BACKGROUND: Color = Color::srgba(0.0, 0.0, 0.0, 0.45);
pub const PICKER_SLOT: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);
pub const PICKER_SLOT_HOVERED: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);

/// Debug screen (F3): text in the top-left corner on a dark box
pub const DEBUG_FONT_SIZE: f32 = 14.0;
pub const DEBUG_MARGIN: f32 = 8.0;
pub const DEBUG_PADDING: f32 = 6.0;
pub const DEBUG_BACKGROUND: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
