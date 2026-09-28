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

/// Pause menu (Esc): a list down the left over the dimmed game, and a panel for
/// Help and Settings beside it. The pixel font is sharp at multiples of 16 px
pub const MENU_DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.9);
pub const MENU_MARGIN: f32 = 48.0;
pub const MENU_LIST_WIDTH: f32 = 240.0;
pub const MENU_PANEL_MAX_WIDTH: f32 = 900.0;
pub const MENU_TITLE_SIZE: f32 = 32.0;
pub const MENU_BUTTON_SIZE: f32 = 32.0;
pub const MENU_BUTTON_GAP: f32 = 12.0;
/// Text: normal, dim (unselected choices, list markers), headings
pub const MENU_TEXT: Color = Color::srgb(0.95, 0.95, 0.95);
pub const MENU_TEXT_DIM: Color = Color::srgb(0.6, 0.6, 0.6);
pub const MENU_HEADING: Color = Color::srgb(1.0, 0.85, 0.45);
/// The pixel font has no bold or italic faces, so those are coloured instead
pub const MENU_BOLD: Color = Color::WHITE;
pub const MENU_ITALIC: Color = Color::srgb(0.7, 0.85, 1.0);
pub const MENU_CODE: Color = Color::srgb(0.75, 1.0, 0.7);
pub const MENU_CODE_BACKGROUND: Color = Color::srgba(0.0, 0.0, 0.0, 0.4);
pub const MENU_SCROLLBAR: Color = Color::srgba(1.0, 1.0, 1.0, 0.35);
pub const MENU_SCROLLBAR_WIDTH: f32 = 4.0;

/// Help page text
pub const HELP_TEXT_SIZE: f32 = 16.0;
pub const HELP_TITLE_SIZE: f32 = 32.0;
/// Space between paragraphs, headings and so on
pub const HELP_BLOCK_GAP: f32 = 12.0;
/// How far list items are indented, per level
pub const HELP_INDENT: f32 = 16.0;
/// Pixels scrolled per step of the mouse wheel
pub const HELP_SCROLL_SPEED: f32 = 48.0;
