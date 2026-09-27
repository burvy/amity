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

/// Sun shadows: how far from the camera they reach (metres), and how many
/// cascades (more gives sharper shadows near the camera, at some GPU cost)
pub const SHADOW_DISTANCE: f32 = 160.0;
pub const SHADOW_CASCADES: usize = 4;
/// Metres covered by the first, sharpest cascade
pub const SHADOW_FIRST_CASCADE: f32 = 12.0;

/// Auto-exposure, like eyes adjusting: the range of scene brightness it can
/// adapt to (in stops, log2 of luminance: daylight ~13, moonlight ~-8), and how
/// fast it adapts in stops per second
pub const EXPOSURE_RANGE: std::ops::RangeInclusive<f32> = -12.0..=15.0;
pub const EXPOSURE_BRIGHTEN_SPEED: f32 = 3.0;
pub const EXPOSURE_DARKEN_SPEED: f32 = 1.0;
