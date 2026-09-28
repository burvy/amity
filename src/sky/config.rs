//! Light levels and camera response, in real units. Illuminance is in lux, the
//! light falling on a surface facing the source: direct sun ~100,000, overcast
//! day ~1,000, full moon ~0.25

use bevy::light::light_consts::lux;

/// Sunlight above the atmosphere. The atmosphere dims and reddens it on its way
/// down (to ~100,000 lux at noon, far less at sunset), so this is the raw value
pub const SUN_LUX: f32 = lux::RAW_SUNLIGHT;

/// Moonlight above the atmosphere at full moon, scaled down by how much of the
/// moon's face is lit
pub const FULL_MOON_LUX: f32 = 0.3;

/// Metres covered by the first, sharpest shadow cascade. How many cascades and
/// how far they reach is the player's Shadows setting (`settings`)
pub const SHADOW_FIRST_CASCADE: f32 = 12.0;

/// The camera's exposure, fixed (in EV100, like a real camera's setting). 15 is
/// made for full sunlight, so daylight looks right and nights are really dark:
/// light there comes from light sources (flashlights, later). Lower is brighter,
/// each 1 doubling it. No eye adjustment, on purpose
pub const EXPOSURE_EV100: f32 = 15.0;
