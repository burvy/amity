// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Ported to Rust from the "Advanced Terrain Erosion Filter" by Rune Skovbo
// Johansen, (c) 2025, https://www.shadertoy.com/view/wXcfWn, explained at
// https://blog.runevision.com/2026/03/fast-and-gorgeous-erosion-filter.html
// (via the C# transcription in https://github.com/lpmitchell/AdvancedTerrainErosion).
// Only this file is under the MPL; changes to it must stay under the MPL.

//! The erosion filter: carves gullies and ridges into a height field, the way
//! running water would, without simulating any water.
//!
//! # How it works
//!
//! Water runs downhill, so gullies run along the slope. For every point, on its
//! own (no neighbours, no simulation, so any chunk can be generated alone):
//!
//! 1. **Stripes along the slope.** `phacelle_noise` makes a stripe pattern whose
//!    stripes run downhill: a cosine wave across the slope gives the height (ridge,
//!    gully, ridge...), and the matching sine wave gives its slope.
//! 2. **Cells, blended.** One stripe pattern everywhere would be too regular, so
//!    space is split into cells (like Worley noise), each with a randomly placed
//!    pivot, and the 4 x 4 nearest cells' stripes are blended by distance.
//! 3. **Octaves.** Like fBm, each octave is smaller and weaker. Each octave's
//!    stripes follow the slope *including the gullies already carved*, so smaller
//!    gullies branch off bigger ones.
//! 4. **Masks.** Gullies fade out on flat ground (`onset`), and each octave limits
//!    where the next can cut, so small gullies don't scar big ridges (`detail`).
//!    Peaks and valley floors are pulled towards a "fade target" (-1 valley, 1
//!    peak) so the big shapes survive.
//!
//! Units: positions and heights in metres, slopes in metres per metre.

use std::f64::consts::TAU;

use bevy::math::{DVec2, DVec3, DVec4};

use super::settings::ErosionSettings;

/// The result of eroding one point
#[derive(Clone, Copy, Debug)]
pub struct Eroded {
    /// Metres to add to the height
    pub height: f64,
    /// -1 in creases (gullies, valleys) .. 1 on ridges. Not used yet: meant for
    /// surface rules (gravel down gully floors) and drainage later
    #[allow(dead_code)]
    pub ridge: f64,
}

/// Erodes the point `p` (metres) of a terrain with slope `slope` (metres per
/// metre, pointing uphill) there. `fade_target` is -1 in valleys and 1 on peaks
/// (overshooting is fine). `octaves` may be fewer than the settings' to compute
/// only the biggest ones (the first octaves never depend on later ones).
pub fn erode(
    settings: &ErosionSettings,
    seed: i32,
    p: DVec2,
    slope: DVec2,
    fade_target: f64,
    octaves: usize,
) -> Eroded {
    let (delta, magnitude, ridge) = erosion_filter(settings, seed, p, slope, fade_target, octaves);
    // lower the terrain by a share of the total erosion (`height_offset.x`),
    // optionally more on peaks than in valleys (`height_offset.y`)
    let offset_factor = lerp(settings.height_offset[0], -fade_target, settings.height_offset[1]);
    Eroded {
        height: delta + offset_factor * magnitude,
        ridge,
    }
}

/// The most metres that octaves `from..` can move the height by, in either
/// direction: each octave moves it at most its strength, and adds its strength to
/// the offset (see `erode`). Used to bound what leaving them out can change
pub fn octave_bound(settings: &ErosionSettings, from: usize) -> f64 {
    let mut strength = settings.strength * settings.scale;
    let mut bound = 0.0;
    for octave in 0..settings.octaves {
        if octave >= from {
            bound += strength * (1.0 + settings.height_offset[0].abs().max(1.0));
        }
        strength *= settings.gain;
    }
    bound
}

/// Metres between the gullies of octave `octave`
pub fn octave_spacing(settings: &ErosionSettings, octave: usize) -> f64 {
    settings.scale * settings.cell_scale / settings.lacunarity.powi(octave as i32)
}

/// The filter itself. Returns (height delta, total strength of the octaves used,
/// ridge map value)
fn erosion_filter(
    settings: &ErosionSettings,
    seed: i32,
    p: DVec2,
    input_slope: DVec2,
    fade_target: f64,
    octaves: usize,
) -> (f64, f64, f64) {
    let rounding = DVec4::from_array(settings.rounding);
    let onset = DVec4::from_array(settings.onset);
    let assumed_slope = DVec2::from_array(settings.assumed_slope);

    let mut strength = settings.strength * settings.scale;
    let mut fade_target = fade_target.clamp(-1.0, 1.0);

    // height in x, slope in yz; the height starts at 0, so what's added is the delta
    let mut height_and_slope = DVec3::new(0.0, input_slope.x, input_slope.y);
    let mut freq = 1.0 / (settings.scale * settings.cell_scale);
    let slope_length = input_slope.length().max(1e-10);
    let mut magnitude = 0.0;
    let mut rounding_mult = 1.0;

    let rounding_for_input =
        lerp(rounding.y, rounding.x, clamp01(fade_target + 0.5)) * rounding.z;
    // The combined accumulating mask, based first on the initial slope, and
    // later on the slope of each octave too
    let mut combi_mask = ease_out(smooth_start(
        slope_length * onset.x,
        rounding_for_input * onset.x,
    ));

    let mut ridge_map_combi_mask = ease_out(slope_length * onset.z);
    let mut ridge_map_fade_target = fade_target;

    // The slope that sets gully directions: a mix of the actual slope and an
    // assumed one, so gentle and steep ground both get well-formed gullies
    let mut gully_slope = input_slope.lerp(
        input_slope / slope_length * assumed_slope.x,
        assumed_slope.y,
    );

    for _ in 0..octaves.min(settings.octaves) {
        // gullies' height offset (cosine) and slope (sine times direction)
        let phacelle = phacelle_noise(
            p * freq,
            gully_slope.normalize_or_zero(),
            settings.cell_scale,
            0.25,
            settings.normalization,
            seed,
        );
        // times freq since p was multiplied by freq; negated since slope
        // directions here point downhill
        let direction = DVec2::new(phacelle.z, phacelle.w) * -freq;

        // how much this octave slopes, 0..1
        let sloping = phacelle.y.abs();

        // the unmasked, normalized slope feeds the next octaves' directions
        gully_slope += phacelle.y.signum() * direction * strength * settings.gully_weight;

        // height offset (-1..1) in x, slope in yz
        let gullies = DVec3::new(phacelle.x, phacelle.y * direction.x, phacelle.y * direction.y);

        // fade the gullies towards the fade target where the mask is low
        let faded = DVec3::new(fade_target, 0.0, 0.0)
            .lerp(gullies * settings.gully_weight, combi_mask);

        height_and_slope += faded * strength;
        magnitude += strength;

        // the next octave fades towards this one's shape
        fade_target = faded.x;

        // tighten the mask with this octave's slope
        let rounding_for_octave =
            lerp(rounding.y, rounding.x, clamp01(phacelle.x + 0.5)) * rounding_mult;
        let new_mask = ease_out(smooth_start(
            sloping * onset.y,
            rounding_for_octave * onset.y,
        ));
        combi_mask = pow_inv(combi_mask, settings.detail) * new_mask;

        ridge_map_fade_target = lerp(ridge_map_fade_target, gullies.x, ridge_map_combi_mask);
        ridge_map_combi_mask *= ease_out(sloping * onset.w);

        strength *= settings.gain;
        freq *= settings.lacunarity;
        rounding_mult *= rounding.w;
    }

    let ridge = ridge_map_fade_target * (1.0 - ridge_map_combi_mask);
    (height_and_slope.x, magnitude, ridge)
}

/// "Phacelle" noise: stripes running along `norm_dir`, blended over a 4 x 4 block
/// of randomly offset cells. Returns the normalized cosine wave (x), sine wave (y),
/// and the stripes' direction vector (zw), which times the sine gives the slope
/// of the cosine
fn phacelle_noise(
    p: DVec2,
    norm_dir: DVec2,
    freq: f64,
    offset: f64,
    normalization: f64,
    seed: i32,
) -> DVec4 {
    // a vector across the stripes, as long as their frequency (times tau)
    let side_dir = DVec2::new(-norm_dir.y, norm_dir.x) * freq * TAU;
    let offset = offset * TAU;

    let p_int = p.floor();
    let p_frac = p - p_int;
    let mut phase_dir = DVec2::ZERO;
    let mut weight_sum = 0.0;

    for i in -1..=2 {
        for j in -1..=2 {
            let grid_offset = DVec2::new(i as f64, j as f64);
            // this cell's point: its grid corner plus a random offset of up to 0.5
            let grid_point = p_int + grid_offset;
            let random_offset = hash2(grid_point, seed) * 0.5;
            let from_cell_point = p_frac - grid_offset - random_offset;

            // bell-shaped weight: 1 at the cell point, 0 at distance 1.5 (the
            // nearest a cell outside the 4 x 4 block can be)
            let sqr_dist = from_cell_point.length_squared();
            let weight = ((-sqr_dist * 2.0).exp() - 0.01111).max(0.0);
            weight_sum += weight;

            // a wave rising along side_dir, at the stripes' frequency
            let wave_input = from_cell_point.dot(side_dir) + offset;
            phase_dir += DVec2::new(wave_input.cos(), wave_input.sin()) * weight;
        }
    }

    let interpolated = phase_dir / weight_sum;
    // blending can shrink the waves; normalize them back (partly, below the
    // `normalization` threshold)
    let magnitude = interpolated.length().max(1.0 - normalization);
    let waves = interpolated / magnitude;
    DVec4::new(waves.x, waves.y, side_dir.x, side_dir.y)
}

/// A pseudo-random vector in -1..1 per grid point, varied by `seed`
fn hash2(x: DVec2, seed: i32) -> DVec2 {
    let k = DVec2::new(1.0 / std::f64::consts::PI, (-1.0f64).exp());
    let seed_offset = seed as f64 * DVec2::new(0.06711056, 0.00583715);
    let x = (x + seed_offset) * k + DVec2::new(k.y, k.x);
    let t = fract(x.x * x.y * (x.x + x.y));
    let inner = 16.0 * k * t;
    DVec2::new(fract(inner.x), fract(inner.y)) * 2.0 - 1.0
}

fn fract(x: f64) -> f64 {
    x - x.floor()
}

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// 1 - (1 - t)^power
fn pow_inv(t: f64, power: f64) -> f64 {
    1.0 - (1.0 - clamp01(t)).powf(power)
}

/// Rises fast, then flattens towards 1
fn ease_out(t: f64) -> f64 {
    let v = 1.0 - clamp01(t);
    1.0 - v * v
}

/// Like `t`, but starting smoothly (quadratically) over the first `smoothing`
fn smooth_start(t: f64, smoothing: f64) -> f64 {
    if t >= smoothing {
        t - 0.5 * smoothing
    } else {
        0.5 * t * t / smoothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> ErosionSettings {
        crate::world::generation::settings::WorldGenSettings::default().erosion
    }

    #[test]
    fn flat_ground_is_left_alone_apart_from_the_offset() {
        let settings = settings();
        let eroded = erosion_filter(&settings, 0, DVec2::new(123.0, -45.0), DVec2::ZERO, 0.0, 5);
        // no slope: every octave is fully faded to the fade target (0)
        assert!(eroded.0.abs() < 1e-9);
    }

    #[test]
    fn slopes_get_carved_and_stay_within_bounds() {
        let settings = settings();
        let bound = octave_bound(&settings, 0);
        let mut carved = 0;
        for i in 0..200 {
            let p = DVec2::new(i as f64 * 7.3, i as f64 * -3.1);
            let eroded = erode(&settings, 0, p, DVec2::new(0.6, 0.2), 0.0, settings.octaves);
            assert!(eroded.height.abs() <= bound, "{} > {bound}", eroded.height);
            if eroded.height.abs() > 0.5 {
                carved += 1;
            }
        }
        assert!(carved > 100, "only {carved} of 200 points eroded");
    }

    #[test]
    fn fewer_octaves_differ_by_at_most_the_bound_of_the_rest() {
        let settings = settings();
        for i in 0..100 {
            let p = DVec2::new(i as f64 * 11.7, i as f64 * 5.3);
            let slope = DVec2::new(0.4, -0.5);
            let all = erode(&settings, 0, p, slope, 0.3, settings.octaves).height;
            let first_two = erode(&settings, 0, p, slope, 0.3, 2).height;
            assert!((all - first_two).abs() <= octave_bound(&settings, 2) + 1e-9);
        }
    }
}
