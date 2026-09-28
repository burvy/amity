//! Surface rules: what the top of the ground is made of in each column.
//!
//! Each column is described by a few facts (its altitude, how steep it is,
//! whether it's under water or just above some). The rules in
//! `SurfaceSettings::rules` are checked top to bottom, and the first whose
//! `When` matches decides the column's layers: the blocks from the top of the
//! ground down, with stone below them. Moving a rule up the list gives it
//! priority.

use super::settings::{SurfaceRule, When};

/// What the rules can ask about one column
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Facts {
    /// Ground height in metres, before rounding to blocks, plus the wobble (see
    /// `SurfaceSettings::wobble`)
    pub altitude: f64,
    /// Steepest angle of the ground, in degrees: 0 flat, 45 one block up per block
    /// across, 90 a wall
    pub slope: f64,
    /// Water stands on this column
    pub under_water: bool,
    /// Metres the ground is above the nearest water surface around it, if there
    /// is water around it
    pub above_water: Option<f64>,
}

/// The steepest slope, in degrees, for a ground that rises `rise_x` metres
/// between the columns either side along x (2 blocks apart), and `rise_z` along z.
///
/// The gradient along x is rise_x / run; along both axes, the steepest direction
/// has gradient √(gx² + gz²), and the angle of that is atan of it.
/// e.g. rising 1 m over the 1 m between the neighbours either side: 45°
pub fn slope_degrees(rise_x: f64, rise_z: f64, run: f64) -> f64 {
    let (gx, gz) = (rise_x / run, rise_z / run);
    (gx * gx + gz * gz).sqrt().atan().to_degrees()
}

impl When {
    /// Whether a column with these facts meets every condition
    pub fn matches(&self, facts: &Facts) -> bool {
        let within = |(min, max): (f64, f64), value: f64| min <= value && value < max;
        within(self.altitude, facts.altitude)
            && within(self.slope, facts.slope)
            && self
                .under_water
                .is_none_or(|under_water| under_water == facts.under_water)
            // asks for water nearby: there must be some, at the right distance below
            && self.above_water.is_none_or(|range| {
                facts
                    .above_water
                    .is_some_and(|above| within(range, above))
            })
    }
}

/// Which rule a column follows: the index of the first that matches, or
/// `rules.len()` if none do (bare stone)
pub fn pick(rules: &[SurfaceRule], facts: &Facts) -> usize {
    rules
        .iter()
        .position(|rule| rule.when.matches(facts))
        .unwrap_or(rules.len())
}

#[cfg(test)]
mod tests {
    use amity_protocol::world::block::Block;

    use super::*;

    fn flat_land(altitude: f64) -> Facts {
        Facts {
            altitude,
            slope: 0.0,
            under_water: false,
            above_water: None,
        }
    }

    #[test]
    fn the_first_matching_rule_wins() {
        let rules = vec![
            SurfaceRule {
                when: When {
                    altitude: (50.0, f64::MAX),
                    ..When::default()
                },
                layers: vec![(Block::SNOW, 1)],
            },
            SurfaceRule {
                when: When::default(),
                layers: vec![(Block::GRASS, 1)],
            },
        ];
        assert_eq!(pick(&rules, &flat_land(80.0)), 0);
        assert_eq!(pick(&rules, &flat_land(10.0)), 1);
        // nothing matches: past the end, meaning bare stone
        assert_eq!(pick(&rules[..1], &flat_land(10.0)), 1);
    }

    #[test]
    fn a_one_to_one_ramp_is_45_degrees() {
        assert!((slope_degrees(1.0, 0.0, 1.0) - 45.0).abs() < 1e-9);
        assert_eq!(slope_degrees(0.0, 0.0, 1.0), 0.0);
    }

    #[test]
    fn asking_for_water_nearby_needs_water_nearby() {
        let beach = When {
            above_water: Some((0.0, 1.5)),
            ..When::default()
        };
        let dry = flat_land(1.0);
        assert!(!beach.matches(&dry));
        assert!(beach.matches(&Facts {
            above_water: Some(1.0),
            ..dry
        }));
        assert!(!beach.matches(&Facts {
            above_water: Some(3.0),
            ..dry
        }));
    }
}
