//! Lakes: where water collects on land, at any height.
//!
//! # The idea
//!
//! Rain collects in low ground until it overflows at the lowest point of the
//! basin's rim (its "spill point"). So a lake's surface is at its spill height,
//! whatever the altitude. Basins connected to the ocean fill to sea level instead.
//!
//! # Priority-flood (Barnes et al., 2014)
//!
//! Works out every cell's water level in one pass over a height grid:
//!
//! 1. Start from the drains: ocean cells (at sea level) and the grid's edge
//!    (water there is assumed to flow away).
//! 2. Repeatedly take the *lowest* cell reached so far (a priority queue), and
//!    reach each unvisited neighbour. Its water level is the higher of its own
//!    ground and the level of the cell it was reached from.
//! 3. Because the lowest cell always goes next, water "climbs" out of a basin only
//!    over its lowest rim point, so everything inside ends up at the spill height.
//! 
//! Where the level ends up above the ground, the cell is under water.
//!
//! # Regions
//!
//! A basin's level depends on its whole rim, so one chunk can't work it out on
//! its own. The world is split into square regions (`Lakes::region` metres), and
//! each is flooded on a coarse grid (`Lakes::cell` metres per cell) together with a
//! margin around it (`Lakes::margin`), so lakes near its edge see their whole rim.
//!
//! Cutting the grid off at the window's edge can only make water *lower* (the edge
//! is a drain), never higher. So near a region's border, where the neighbouring
//! region's window covers the same cells, each cell takes the *highest* level any
//! covering window gives it: the least cut-off view. Neighbouring cells on either
//! side of a border then look at the same windows and agree, so there's never a
//! step in the water at a border. (A lake far wider than the margin can still be
//! a bit too low, since no window sees its whole rim.)
//!
//! A region is computed the first time a chunk inside it needs it, on that
//! chunk's background thread, and cached.
//!
//! # No walls of water
//!
//! Water is drawn wherever it meets air. If a column had water up to 6 m while
//! the column beside it was dry ground at 2 m, the water's side would show as a
//! flat wall hanging in the air. Two rules together make that impossible:
//!
//! - **A cell's height is the lowest the ground can be anywhere inside it**, not
//!   the height at its centre (see `Lakes::compute`).
//! - **A column takes its own cell's water level.**
//!
//! Priority-flood guarantees that a dry cell next to a lake has a height at or
//! above the lake's surface (else the water would have reached it). With cell
//! heights being lower bounds, every column in that dry cell is at or above the
//! surface too, so the water always meets ground, never air.

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    sync::{Arc, Mutex, OnceLock},
};

use bevy::{math::DVec2, prelude::*};

use super::settings::LakeSettings;

/// Metres a cell's height is lowered beyond the lowest of its 5 samples, for
/// dips in the ground between the sample points
const SUBCELL_SLACK: f32 = 2.0;

/// A cell's water level when it has no water. Lower than any real level, so the
/// highest of several cells' levels is simply `max`
pub const NO_WATER: f32 = f32::NEG_INFINITY;

/// Samples the terrain for the lake grid, many points at once
pub trait SampleGrid {
    /// The `count` x `count` points `first + (i, j) * spacing` (metres), row by
    /// row. Heights must be lower bounds: see `Lakes::compute`. Only points with
    /// `i + j` even are used (cell corners and centres); the others may be skipped
    fn sample_grid(&self, first: DVec2, spacing: f64, count: usize) -> Vec<CellSample>;
}

/// What the flood needs to know about one coarse cell, or one sample point
#[derive(Clone, Copy)]
pub struct CellSample {
    /// ground height, metres
    pub height: f32,
    /// true ocean (continentalness below the coast): a drain at sea level
    pub ocean: bool,
}

/// The lakes of the whole world, computed a region at a time
pub struct Lakes {
    /// metres per cell
    cell: f64,
    /// cells along a region's edge
    region_cells: i32,
    /// cells of margin flooded around each region
    margin_cells: i32,
    /// metres: lakes shallower than this are dropped
    min_depth: f32,
    /// metres: the ocean's surface
    sea_level: f32,
    /// Finished (or being computed) regions, by region coordinate. The `OnceLock`
    /// lets the first chunk that needs a region compute it while other chunks that
    /// need the same region wait for that result instead of computing it again
    regions: Mutex<HashMap<IVec2, Arc<OnceLock<Region>>>>,
}

/// One region's flood result: every cell's water level (metres, or NO_WATER) in
/// its whole window (the region plus its margin), row by row
struct Region {
    levels: Box<[f32]>,
}

impl Lakes {
    pub fn new(settings: &LakeSettings, sea_level: f64) -> Self {
        Self {
            cell: settings.cell,
            region_cells: (settings.region / settings.cell).round() as i32,
            margin_cells: (settings.margin / settings.cell).round() as i32,
            min_depth: settings.min_depth as f32,
            sea_level: sea_level as f32,
            regions: Mutex::default(),
        }
    }

    /// Metres per cell
    pub fn cell_size(&self) -> f64 {
        self.cell
    }

    /// The water level (metres) at (x, z) metres, or NO_WATER: the level of the
    /// cell containing the point. Inside a lake cell, only columns whose ground is
    /// below the level get water, so shorelines still follow the fine terrain.
    /// `sample` gives the terrain, for computing regions not seen yet
    pub fn level_at(&self, x: f64, z: f64, sample: &impl SampleGrid) -> f32 {
        // cell (i, j) covers i * cell .. (i + 1) * cell along x, same for z
        let i = (x / self.cell).floor() as i32;
        let j = (z / self.cell).floor() as i32;
        self.cell_level(IVec2::new(i, j), sample)
    }

    /// The water level of one cell, computing its region first if needed
    /// The highest level any region's window gives this cell (see "Regions"). A
    /// cell deep inside its region is only in its own window; one within the
    /// margin of a border is also in the neighbouring regions' windows
    fn cell_level(&self, cell: IVec2, sample: &impl SampleGrid) -> f32 {
        let home = cell.div_euclid(IVec2::splat(self.region_cells));
        let window = self.region_cells + 2 * self.margin_cells;
        let mut level = NO_WATER;
        for dz in -1..=1 {
            for dx in -1..=1 {
                let region = home + IVec2::new(dx, dz);
                // the cell's position inside this region's window, if it's in it
                let local = cell - (region * self.region_cells - IVec2::splat(self.margin_cells));
                if local.min_element() < 0 || local.max_element() >= window {
                    continue;
                }
                let slot = self.region(region, sample);
                let levels = &slot.get().expect("`region` fills the slot").levels;
                level = level.max(levels[(local.x + local.y * window) as usize]);
            }
        }
        level
    }

    /// A region's flood result, computing it first if nobody has yet
    fn region(
        &self,
        region: IVec2,
        sample: &impl SampleGrid,
    ) -> Arc<OnceLock<Region>> {
        // hold the map's lock only to find (or add) the region's slot, never while
        // computing, so chunks in other regions aren't held up
        let slot = self
            .regions
            .lock()
            .unwrap()
            .entry(region)
            .or_default()
            .clone();
        slot.get_or_init(|| self.compute(region, sample));
        slot
    }

    /// Floods one region together with its margin
    fn compute(&self, region: IVec2, sample: &impl SampleGrid) -> Region {
        // the window: the region plus `margin_cells` on every side
        let window = (self.region_cells + 2 * self.margin_cells) as usize;
        let first = region * self.region_cells - IVec2::splat(self.margin_cells);

        // Each cell's height must be a lower bound on the ground anywhere inside it
        // (see "No walls of water" above). The terrain is sampled at every cell's
        // corners (shared with its neighbours) and centre, which all lie on a grid
        // half a cell apart, and a cell takes the lowest of those 5, minus
        // SUBCELL_SLACK for dips between them. (`sample` already lowers heights by
        // the most the details too fine to sample could.)
        let across = 2 * window + 1;
        let first_point = first.as_dvec2() * self.cell;
        let points = sample.sample_grid(first_point, self.cell / 2.0, across);
        let mut cells = Vec::with_capacity(window * window);
        for j in 0..window {
            for i in 0..window {
                let point = |di: usize, dj: usize| points[(2 * i + di) + (2 * j + dj) * across];
                let lowest = [(0, 0), (2, 0), (0, 2), (2, 2), (1, 1)]
                    .map(|(di, dj)| point(di, dj).height)
                    .into_iter()
                    .fold(f32::INFINITY, f32::min);
                cells.push(CellSample {
                    height: lowest - SUBCELL_SLACK,
                    ocean: point(1, 1).ocean,
                });
            }
        }
        Region {
            levels: flood(&cells, window, self.sea_level, self.min_depth).into_boxed_slice(),
        }
    }
}

/// An entry in the flood's priority queue: a cell and the water level it was
/// reached at. Ordered so the *lowest* level comes out of the heap first
struct Frontier {
    level: f32,
    index: usize,
}

impl Ord for Frontier {
    fn cmp(&self, other: &Self) -> Ordering {
        // reversed, because `BinaryHeap` gives the largest first. Ties go by index
        // so the result never depends on the heap's internal order
        other
            .level
            .total_cmp(&self.level)
            .then(other.index.cmp(&self.index))
    }
}

impl PartialOrd for Frontier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Frontier {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Frontier {}

/// Priority-flood over a `size` x `size` grid (row by row). Returns each cell's
/// water level in metres, or NO_WATER for dry cells. Lakes shallower than
/// `min_depth` are dropped; water connected to the ocean is always kept.
pub fn flood(cells: &[CellSample], size: usize, sea_level: f32, min_depth: f32) -> Vec<f32> {
    let n = size * size;
    // the water level each cell will end up with: its own ground or higher
    let mut level = vec![f32::INFINITY; n];
    let mut done = vec![false; n];
    let mut queue = BinaryHeap::new();

    // Step 1: seed the drains. Grid-edge cells drain at their own height (water
    // there flows off the grid); ocean cells at sea level
    for index in 0..n {
        let (i, j) = (index % size, index / size);
        let edge = i == 0 || j == 0 || i == size - 1 || j == size - 1;
        let cell = cells[index];
        let seed = if cell.ocean {
            Some(cell.height.max(sea_level))
        } else if edge {
            Some(cell.height)
        } else {
            None
        };
        if let Some(seed) = seed {
            level[index] = seed;
            queue.push(Frontier { level: seed, index });
        }
    }

    // Step 2: grow inward from the lowest cell reached so far
    while let Some(Frontier { level: here, index }) = queue.pop() {
        // a cell can be queued more than once (reached from several sides); only
        // its first, lowest, visit counts
        if done[index] {
            continue;
        }
        done[index] = true;
        for neighbour in neighbours(index, size) {
            if done[neighbour] {
                continue;
            }
            // water reaching the neighbour from here stands at least at `here`,
            // and never below the neighbour's own ground
            let reached = here.max(cells[neighbour].height);
            if reached < level[neighbour] {
                level[neighbour] = reached;
                queue.push(Frontier {
                    level: reached,
                    index: neighbour,
                });
            }
        }
    }

    // Step 3: cells whose level is above their ground are under water
    let mut water: Vec<f32> = (0..n)
        .map(|i| {
            if level[i] > cells[i].height {
                level[i]
            } else {
                NO_WATER
            }
        })
        .collect();
    drop_shallow_lakes(&mut water, cells, size, min_depth);
    water
}

/// Removes lakes whose deepest point is shallower than `min_depth`, so the land
/// isn't covered in puddles. A lake is a connected group of water cells with the
/// same level; one touching the ocean is the sea and is always kept.
fn drop_shallow_lakes(water: &mut [f32], cells: &[CellSample], size: usize, min_depth: f32) {
    let mut seen = vec![false; water.len()];
    let mut lake = Vec::new();
    for start in 0..water.len() {
        if seen[start] || water[start] == NO_WATER {
            continue;
        }
        // collect the whole lake with a flood fill from `start`
        let surface = water[start];
        lake.clear();
        lake.push(start);
        seen[start] = true;
        let mut next = 0;
        let (mut deepest, mut sea) = (0.0f32, false);
        while next < lake.len() {
            let index = lake[next];
            next += 1;
            deepest = deepest.max(surface - cells[index].height);
            sea |= cells[index].ocean;
            for neighbour in neighbours(index, size) {
                if !seen[neighbour] && water[neighbour] == surface {
                    seen[neighbour] = true;
                    lake.push(neighbour);
                }
            }
        }
        if !sea && deepest < min_depth {
            for &index in &lake {
                water[index] = NO_WATER;
            }
        }
    }
}

/// The up to 4 cells sharing an edge with `index`
fn neighbours(index: usize, size: usize) -> impl Iterator<Item = usize> {
    let (i, j) = (index % size, index / size);
    [
        (i > 0).then(|| index - 1),
        (i + 1 < size).then(|| index + 1),
        (j > 0).then(|| index - size),
        (j + 1 < size).then(|| index + size),
    ]
    .into_iter()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A grid from a height function, no ocean
    fn grid(size: usize, height: impl Fn(usize, usize) -> f32) -> Vec<CellSample> {
        (0..size * size)
            .map(|index| CellSample {
                height: height(index % size, index / size),
                ocean: false,
            })
            .collect()
    }

    /// A bowl centred on cell (10, 10): ground 10 m everywhere, a square ring wall
    /// 20 m high (4 cells from the centre) with a notch down to 15 m, and a floor
    /// of 5 m inside the ring
    fn bowl(i: usize, j: usize) -> f32 {
        // distance from the centre, in cells, along the further axis
        let d = (i as f32 - 10.0).abs().max((j as f32 - 10.0).abs());
        match d {
            d if d < 4.0 => 5.0,
            4.0 if (i, j) == (10, 6) => 15.0, // the notch
            4.0 => 20.0,
            _ => 10.0,
        }
    }

    #[test]
    fn a_bowl_fills_to_its_lowest_rim_point() {
        let water = flood(&grid(21, bowl), 21, 0.0, 1.0);
        // inside the ring: filled to the notch's height, 15 m
        assert_eq!(water[10 + 10 * 21], 15.0);
        // outside: dry
        assert_eq!(water[1 + 21], NO_WATER);
    }

    #[test]
    fn basins_touching_the_ocean_stay_at_sea_level() {
        // a low valley (-5 m) running into the ocean along row 0: no rim, so it
        // fills only to sea level, not higher. It stops short of the far edge,
        // which would be a drain at the valley's own height
        let mut cells = grid(10, |i, j| {
            if (i == 5 && j < 8) || j == 0 {
                -5.0
            } else {
                3.0
            }
        });
        for cell in cells.iter_mut().take(10) {
            cell.ocean = true;
        }
        let water = flood(&cells, 10, 0.0, 1.0);
        assert_eq!(water[5 + 5 * 10], 0.0);
        assert_eq!(water[2 + 5 * 10], NO_WATER);
    }

    #[test]
    fn shallow_puddles_are_dropped() {
        // a single 10 cm dip
        let water = flood(
            &grid(9, |i, j| if (i, j) == (4, 4) { 9.9 } else { 10.0 }),
            9,
            0.0,
            1.0,
        );
        assert!(water.iter().all(|&level| level == NO_WATER));
    }

    #[test]
    fn overlapping_windows_agree_on_a_lake() {
        // the same bowl seen by two windows, the second shifted 3 cells along x.
        // Both contain the whole bowl, so both must give it the same level
        let a = flood(&grid(21, bowl), 21, 0.0, 1.0);
        let shifted = |i: usize, j| if i + 3 <= 20 { bowl(i + 3, j) } else { 10.0 };
        let b = flood(&grid(21, shifted), 21, 0.0, 1.0);
        // cell (10, 10) in window a is (7, 10) in window b
        assert_eq!(a[10 + 10 * 21], 15.0);
        assert_eq!(b[7 + 10 * 21], 15.0);
    }
}
