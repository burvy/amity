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

/// Haze: fine particles in the air near the ground, which fade distant things
/// toward the colour of the horizon sky (aerial perspective). How far you can
/// see through it at ground level (metres): a thing this far away keeps only
/// 2% of its contrast
pub const HAZE_VISIBILITY: f32 = 2_000.0;
/// How quickly the haze thins with height (metres for it to fall to about a
/// third). A shallow layer: from high ground you see out over it
pub const HAZE_SCALE_HEIGHT: f32 = 300.0;
/// The share of the light haze stops that it scatters rather than absorbs
pub const HAZE_SCATTER_SHARE: f32 = 0.9;
/// How much more haze scatters blue than red (the Angstrom exponent): 0 for
/// grey water droplets like fog, higher for the finer particles of haze
pub const HAZE_ANGSTROM: f32 = 1.3;
/// How strongly haze scatters light onward rather than aside (0 evenly, near
/// 1 all forward): what makes the air glow around the sun
pub const HAZE_ASYMMETRY: f32 = 0.7;

/// Steps in the atmosphere's table of density by height, from the ground to
/// the edge of space. Fine enough to hold the shallow haze layer
pub const ATMOSPHERE_HEIGHT_STEPS: u32 = 4096;
/// How far out aerial perspective is worked out (metres). Covers far terrain
/// to come (LOD), which the haze will show faded in the distance
pub const AERIAL_PERSPECTIVE_DISTANCE: f32 = 2_000.0;
