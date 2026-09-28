//! Structures in the generated world: trees now, rocks and ruins later. Placed
//! the standard way ("features", as Minecraft calls them), so any chunk can be
//! generated on its own, in any order, and still agree with its neighbours:
//!
//! 1. **Candidate spots.** The world's columns are grouped into square cells
//!    (`STRUCTURE_CELL` blocks across). Each cell gets one candidate spot, at a
//!    place in it picked from the seed and the cell's position.
//! 2. **Whether something grows there.** Another number from the seed and the
//!    cell decides whether this cell gets a structure, and which of its
//!    biome's features: each feature gets its share of chances, from how dense
//!    it is (`Feature::per_100m2`). The spot's column must then meet the
//!    feature's conditions (dry, not too steep, on grass...).
//! 3. **Which one, and how turned.** More numbers pick the structure (by the
//!    feature's weights), a quarter turn and a mirroring.
//! 4. **Writing it.** A chunk being generated looks at every candidate spot
//!    close enough for a structure to reach into it, and writes only the part
//!    inside itself. Its neighbours look at the same spots and get the same
//!    answers, so a tree split across chunks lines up.
//!
//! A structure only fills air and water: it never replaces the ground, and
//! where two overlap, the one from the earlier cell (by position) wins. Every
//! chunk visits the cells in the same order, so it's always the same one.

use amity_protocol::world::{block::Block, structure::Orientation, structure::Structure};
use bevy::{platform::collections::HashMap, prelude::*};

use super::{config::STRUCTURE_CELL, settings::Feature, settings::When};
use amity_protocol::world::config::BLOCK_SIZE;

// the packed structure files: `FILES`, a list of (name, text). See build.rs
include!(concat!(env!("OUT_DIR"), "/structures.rs"));

/// A feature, ready to roll for
pub struct Placing {
    /// the structures and their weights
    structures: Vec<(usize, u32)>,
    total_weight: u32,
    /// the chance a cell gets one, 0 to 1
    chance: f64,
    when: When,
    on: Vec<Block>,
}

/// Every structure file, and every biome's features, ready to place
pub struct Structures {
    all: Vec<Structure>,
    /// per biome, in the same order as the settings' biomes
    features: Vec<Vec<Placing>>,
    /// the most any biome's features add up to: cells rolling above it get
    /// nothing wherever they are, so they're dropped without looking at their
    /// column
    most_chance: f64,
    /// blocks: the furthest any structure reaches sideways, and the tallest
    reach: i32,
    height: i32,
}

/// A structure to write: which, where its anchor goes, and how it's turned
pub struct Planted<'a> {
    pub structure: &'a Structure,
    pub anchor: IVec3,
    pub orientation: Orientation,
}

impl Structures {
    /// Reads every packed structure file, and turns the biomes' features into
    /// chances. Panics, saying which, on a feature naming a structure that
    /// doesn't exist, or a density too high for the cells to hold
    pub fn new(biomes: &[&[Feature]]) -> Self {
        let mut names = HashMap::new();
        let mut all = Vec::new();
        for (name, text) in FILES {
            let structure = Structure::parse(text)
                .unwrap_or_else(|e| panic!("assets/structures/{name}.ron: {e}"));
            names.insert(*name, all.len());
            all.push(structure);
        }
        // square metres per cell
        let cell_area = (STRUCTURE_CELL as f64 * BLOCK_SIZE as f64).powi(2);
        let features: Vec<Vec<Placing>> = biomes
            .iter()
            .map(|features| {
                features
                    .iter()
                    .map(|feature| Placing {
                        structures: feature
                            .structures
                            .iter()
                            .map(|&(name, weight)| {
                                let index = *names.get(name).unwrap_or_else(|| {
                                    panic!("a feature uses structure {name:?}, but assets/structures/{name}.ron doesn't exist")
                                });
                                (index, weight)
                            })
                            .collect(),
                        total_weight: feature.structures.iter().map(|&(_, weight)| weight).sum(),
                        chance: feature.per_100m2 * cell_area / 100.0,
                        when: feature.when.clone(),
                        on: feature.on.clone(),
                    })
                    .collect()
            })
            .collect();
        let most_chance = features
            .iter()
            .map(|features: &Vec<Placing>| features.iter().map(|f| f.chance).sum::<f64>())
            .fold(0.0, f64::max);
        assert!(
            most_chance <= 1.0,
            "a biome's features want more than one structure per {STRUCTURE_CELL}-block cell: lower their per_100m2"
        );
        Self {
            reach: all.iter().map(Structure::reach).max().unwrap_or(0),
            height: all.iter().map(Structure::height).max().unwrap_or(0),
            all,
            features,
            most_chance,
        }
    }

    /// The furthest any structure reaches sideways from its spot, in blocks
    pub fn reach(&self) -> i32 {
        self.reach
    }

    /// The most any structure rises above the ground, in blocks
    pub fn height(&self) -> i32 {
        self.height
    }

    /// The cell's candidate spot (block x, z), and its roll (0 to 1). None if
    /// the roll is too high for anything to grow in any biome
    pub fn candidate(&self, seed: u32, cell: IVec2) -> Option<(IVec2, f64)> {
        let roll = unit(hash(seed, cell, 1));
        if roll >= self.most_chance {
            return None;
        }
        let spot = hash(seed, cell, 0);
        let within = IVec2::new(
            (spot % STRUCTURE_CELL as u64) as i32,
            ((spot >> 16) % STRUCTURE_CELL as u64) as i32,
        );
        Some((cell * STRUCTURE_CELL + within, roll))
    }

    /// What grows at a cell's spot, given its column: its biome, the facts its
    /// surface was picked by, its top block, and the height of its ground
    /// (the first block above it). None if nothing does
    pub fn plant(
        &self,
        seed: u32,
        cell: IVec2,
        spot: IVec2,
        roll: f64,
        column: SpotColumn,
    ) -> Option<Planted<'_>> {
        // each feature owns a slice of the rolls, as wide as its chance
        let mut below = 0.0;
        let feature = self.features[column.biome].iter().find(|feature| {
            below += feature.chance;
            roll < below
        })?;
        if !feature.when.matches(&column.facts) {
            return None;
        }
        if !feature.on.is_empty() && !feature.on.contains(&column.top) {
            return None;
        }
        let pick = (hash(seed, cell, 2) % feature.total_weight.max(1) as u64) as u32;
        let mut weights = 0;
        let &(index, _) = feature.structures.iter().find(|&&(_, weight)| {
            weights += weight;
            pick < weights
        })?;
        let turn = hash(seed, cell, 3);
        Some(Planted {
            structure: &self.all[index],
            anchor: IVec3::new(spot.x, column.ground, spot.y),
            orientation: Orientation {
                turns: (turn % 4) as u8,
                mirrored: (turn >> 8) & 1 == 1,
            },
        })
    }
}

/// What `Structures::plant` needs to know about a spot's column
pub struct SpotColumn {
    pub biome: usize,
    pub facts: super::surface::Facts,
    pub top: Block,
    pub ground: i32,
}

/// A well-mixed 64-bit number from the seed, a cell and what it's for
/// (`salt`): the same inputs always give the same number, and nearby cells
/// give unrelated ones (SplitMix64's mixing)
fn hash(seed: u32, cell: IVec2, salt: u32) -> u64 {
    let mut x = (seed as u64) << 32 ^ (salt as u64);
    x ^= (cell.x as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= (cell.y as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// A hash as a number from 0 up to (not including) 1
fn unit(hash: u64) -> f64 {
    (hash >> 11) as f64 / (1u64 << 53) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_structure_file_loads() {
        assert!(!FILES.is_empty());
        for (name, text) in FILES {
            assert!(Structure::parse(text).is_ok(), "{name}");
        }
    }

    #[test]
    fn rolls_are_fixed_by_seed_and_cell_and_spread_out() {
        let cell = IVec2::new(-3, 12);
        assert_eq!(hash(7, cell, 1), hash(7, cell, 1));
        assert_ne!(hash(7, cell, 1), hash(8, cell, 1));
        assert_ne!(hash(7, cell, 1), hash(7, cell + IVec2::X, 1));
        // about half of many rolls fall under a half
        let under = (0..1000)
            .filter(|&i| unit(hash(1, IVec2::new(i, 0), 1)) < 0.5)
            .count();
        assert!((400..600).contains(&under), "{under}");
    }
}
