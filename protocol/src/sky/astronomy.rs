//! Where the sun and moon are in the sky, from the date and time.
//!
//! The same maths as a real sun position calculator, simplified: the sun's
//! declination (how far north or south of the equator it is overhead, which is
//! what makes the seasons) follows the year, its hour angle follows the time of
//! day, and the observer's latitude turns those into a direction in our sky.
//!
//! Directions: +X east, -Z north, +Y up (so in the northern hemisphere the noon
//! sun is to the south, +Z).

use std::f64::consts::TAU;

use bevy::prelude::*;

use super::config::{DAYS_PER_YEAR, LATITUDE_DEG, LUNAR_MONTH_DAYS};

/// Earth's axial tilt: how far the sun's declination swings each year, degrees
const AXIAL_TILT_DEG: f64 = 23.44;

/// The moon's orbit is tilted about 5 degrees against the sun's path
const MOON_INCLINATION_DEG: f64 = 5.14;

/// A unit vector pointing from the ground towards a body at `declination`
/// (radians) and `hour_angle` (radians: 0 when it's highest, negative before),
/// seen from `latitude` (radians)
fn direction(latitude: f64, declination: f64, hour_angle: f64) -> Vec3 {
    let (sin_lat, cos_lat) = latitude.sin_cos();
    let (sin_dec, cos_dec) = declination.sin_cos();
    let (sin_h, cos_h) = hour_angle.sin_cos();
    // standard equatorial -> horizontal coordinates
    let up = sin_lat * sin_dec + cos_lat * cos_dec * cos_h;
    let east = -cos_dec * sin_h;
    let north = cos_lat * sin_dec - sin_lat * cos_dec * cos_h;
    Vec3::new(east as f32, up as f32, -north as f32).normalize()
}

/// The sun's declination (radians) on day `days` (since 1 January of year 0):
/// its highest in midsummer (day ~172), lowest in midwinter
fn sun_declination(days: f64) -> f64 {
    let year_fraction = (days - 81.0 * DAYS_PER_YEAR / 365.0) / DAYS_PER_YEAR;
    AXIAL_TILT_DEG.to_radians() * (TAU * year_fraction).sin()
}

/// The direction towards the sun at `days` (whole days since 1 January of year 0,
/// plus the fraction of the current day)
pub fn sun_direction(days: f64) -> Vec3 {
    // noon (x.5) is hour angle 0
    let hour_angle = TAU * (days.fract() - 0.5);
    direction(LATITUDE_DEG.to_radians(), sun_declination(days), hour_angle)
}

/// How far through its cycle the moon is: 0 new moon (next to the sun, dark),
/// 0.5 full moon (opposite the sun), back to 1
pub fn moon_phase(days: f64) -> f64 {
    // a new moon on day 0
    (days / LUNAR_MONTH_DAYS).fract()
}

/// The fraction of the moon's face that's lit, 0 (new) to 1 (full)
pub fn moon_illumination(days: f64) -> f64 {
    (1.0 - (TAU * moon_phase(days)).cos()) / 2.0
}

/// The direction towards the moon. It trails the sun by its phase (a full moon
/// rises as the sun sets), and sits roughly where the sun will be half a year on
/// at full moon, give or take the tilt of its orbit
pub fn moon_direction(days: f64) -> Vec3 {
    let phase = moon_phase(days);
    let hour_angle = TAU * (days.fract() - 0.5 - phase);
    let declination = sun_declination(days + phase * DAYS_PER_YEAR)
        + MOON_INCLINATION_DEG.to_radians() * (TAU * days / 27.2).sin();
    direction(LATITUDE_DEG.to_radians(), declination, hour_angle)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EQUINOX: f64 = 81.0;
    const MIDSUMMER: f64 = 172.0;

    #[test]
    fn the_noon_sun_is_south_and_as_high_as_the_latitude_allows() {
        // at the equinox the noon sun stands 90 - latitude above the horizon
        let noon = sun_direction(EQUINOX + 0.5);
        let elevation = noon.y.asin().to_degrees() as f64;
        assert!((elevation - (90.0 - LATITUDE_DEG)).abs() < 0.5, "{elevation}");
        assert!(noon.z > 0.0 && noon.x.abs() < 1e-3, "{noon}");
    }

    #[test]
    fn it_rises_east_sets_west_and_is_down_at_midnight() {
        assert!(sun_direction(EQUINOX + 0.3).x > 0.0);
        assert!(sun_direction(EQUINOX + 0.7).x < 0.0);
        assert!(sun_direction(EQUINOX).y < 0.0);
    }

    #[test]
    fn summer_days_are_higher_than_winter_ones() {
        let summer = sun_direction(MIDSUMMER + 0.5).y;
        let winter = sun_direction(MIDSUMMER + 182.5 + 0.5).y;
        assert!(summer > winter + 0.5, "{summer} vs {winter}");
    }

    #[test]
    fn a_full_moon_is_opposite_the_sun() {
        let full = LUNAR_MONTH_DAYS / 2.0;
        assert!((moon_illumination(full) - 1.0).abs() < 1e-9);
        assert!(moon_illumination(0.0) < 1e-9);
        // near midnight at full moon the moon is up and the sun is down
        let night = full.floor() + 1.0;
        assert!(moon_direction(night).y > 0.0 && sun_direction(night).y < 0.0);
    }
}
