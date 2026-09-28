//! Working out the horizon map, on a background thread: the ground's height in
//! every cell, then from each cell, how high the terrain rises around it in
//! each of 8 directions

use std::f32::consts::{FRAC_PI_2, TAU};

use amity_protocol::world::{
    chunk::ChunkSky,
    config::{BLOCK_SIZE, CHUNK_SIZE},
};
use bevy::{platform::collections::HashMap, prelude::*};

use super::config::*;

/// Directions looked in: every 45 degrees, starting along +X and turning
/// towards +Z. The shader picks between them by the sun's direction the same way
pub const DIRECTIONS: usize = 8;

/// No ground known here (not loaded)
const UNKNOWN: f32 = f32::NEG_INFINITY;

/// Everything the background task needs: each loaded chunk's view of the sky
/// (which says where its columns' ground is), and the map's corner cell
pub struct Snapshot {
    pub skies: Vec<(IVec3, ChunkSky)>,
    /// the cell at the map's (0, 0)
    pub corner: IVec2,
}

/// The map's texture data: 2 layers of MAP_CELLS x MAP_CELLS, 4 bytes per cell
/// each (directions 0-3, then 4-7). Each byte is the horizon's height above
/// flat, 0 (flat) to 255 (straight up). A cell's texel is its world cell
/// position wrapped to the map size, so the shader finds it from a world
/// position alone, without knowing where the map is centred
pub fn horizon_texture(snapshot: &Snapshot) -> Vec<u8> {
    let heights = cell_heights(snapshot);
    let n = MAP_CELLS as usize;
    let mut data = vec![0u8; n * n * DIRECTIONS];
    for z in 0..MAP_CELLS {
        for x in 0..MAP_CELLS {
            let angles = horizon(&heights, IVec2::new(x, z));
            // where this cell's texel is, wrapped
            let texel = (snapshot.corner + IVec2::new(x, z)).rem_euclid(IVec2::splat(MAP_CELLS));
            let texel = (texel.x + texel.y * MAP_CELLS) as usize;
            for (direction, angle) in angles.iter().enumerate() {
                let layer = direction / 4;
                let byte = (layer * n * n + texel) * 4 + direction % 4;
                data[byte] = (angle / FRAC_PI_2 * 255.0).round() as u8;
            }
        }
    }
    data
}

/// Each cell's ground height in metres: the highest of its block columns.
/// A column's height is where its sky opens: the one chunk it opens inside
/// says exactly; failing that, a chunk covered all the way up says the ground
/// is at least that high; failing that (open in every loaded chunk), it's below
/// the lowest one, and the cell is left unknown
fn cell_heights(snapshot: &Snapshot) -> Vec<f32> {
    let n = MAP_CELLS as usize;
    let mut heights = vec![UNKNOWN; n * n];

    // the chunks of each chunk column
    let mut columns: HashMap<IVec2, Vec<(i32, &ChunkSky)>> = HashMap::default();
    for (pos, sky) in &snapshot.skies {
        columns.entry(pos.xz()).or_default().push((pos.y, sky));
    }

    let first_block = snapshot.corner * CELL_BLOCKS;
    let blocks = MAP_CELLS * CELL_BLOCKS;
    for (column, chunks) in &columns {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let block = *column * CHUNK_SIZE + IVec2::new(x, z);
                let in_map = block - first_block;
                if in_map.cmplt(IVec2::ZERO).any() || in_map.cmpge(IVec2::splat(blocks)).any() {
                    continue;
                }
                // the highest place this column is known to be covered up to
                let ground = chunks
                    .iter()
                    .map(|(y, sky)| (y * CHUNK_SIZE, sky.open_from(x, z)))
                    .filter(|&(_, open)| open > 0)
                    .map(|(bottom, open)| bottom + open)
                    .max();
                let Some(ground) = ground else {
                    continue;
                };
                let cell = in_map / CELL_BLOCKS;
                let height = &mut heights[(cell.x + cell.y * MAP_CELLS) as usize];
                *height = height.max(ground as f32 * BLOCK_SIZE);
            }
        }
    }
    heights
}

/// How high the terrain rises around `cell`, in each direction: the steepest
/// angle up to any cell up to SEARCH_CELLS away, in radians (0 to pi/2).
/// Unknown ground doesn't count
fn horizon(heights: &[f32], cell: IVec2) -> [f32; DIRECTIONS] {
    let at = |p: IVec2| {
        if p.cmplt(IVec2::ZERO).any() || p.cmpge(IVec2::splat(MAP_CELLS)).any() {
            UNKNOWN
        } else {
            heights[(p.x + p.y * MAP_CELLS) as usize]
        }
    };
    let own = at(cell);
    if own == UNKNOWN {
        return [0.0; DIRECTIONS];
    }
    let cell_metres = (CELL_BLOCKS as f32) * BLOCK_SIZE;
    std::array::from_fn(|direction| {
        let angle = direction as f32 / DIRECTIONS as f32 * TAU;
        let step = Vec2::new(angle.cos(), angle.sin());
        let mut steepest = 0.0f32;
        for distance in 1..=SEARCH_CELLS {
            let there = at((cell.as_vec2() + step * distance as f32).round().as_ivec2());
            let rise = there - own - SELF_SHADOW_BIAS;
            steepest = steepest.max(rise / (distance as f32 * cell_metres));
        }
        steepest.atan()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A map of flat ground at `height` blocks, with `wall` cells raised to
    /// `wall_height` blocks
    fn heights(height: f32, wall: impl Fn(IVec2) -> bool, wall_height: f32) -> Vec<f32> {
        (0..MAP_CELLS * MAP_CELLS)
            .map(|i| IVec2::new(i % MAP_CELLS, i / MAP_CELLS))
            .map(|cell| if wall(cell) { wall_height } else { height } * BLOCK_SIZE)
            .collect()
    }

    #[test]
    fn flat_ground_has_no_horizon() {
        let flat = heights(10.0, |_| false, 0.0);
        assert_eq!(horizon(&flat, IVec2::splat(50)), [0.0; DIRECTIONS]);
    }

    #[test]
    fn a_wall_rises_only_in_its_direction() {
        // a wall along x = 60, 40 m (80 blocks) tall; standing 10 cells (20 m) west of it
        let walled = heights(0.0, |cell| cell.x == 60, 80.0);
        let angles = horizon(&walled, IVec2::new(50, 50));
        // towards +X (direction 0): up at about atan((40 - 1) / 20)
        let expected = ((40.0 - SELF_SHADOW_BIAS) / 20.0).atan();
        assert!((angles[0] - expected).abs() < 1e-4, "{}", angles[0]);
        // the opposite way (direction 4): nothing
        assert_eq!(angles[4], 0.0);
    }

    #[test]
    fn ground_comes_from_where_the_sky_opens() {
        // one chunk at the map's corner, open from local height 10 everywhere:
        // the ground is at block 10, 5 m
        let snapshot = Snapshot {
            skies: vec![(IVec3::ZERO, ChunkSky::Uniform(10))],
            corner: IVec2::ZERO,
        };
        let heights = cell_heights(&snapshot);
        assert_eq!(heights[0], 10.0 * BLOCK_SIZE);
        // cells outside that chunk know nothing
        assert_eq!(heights[(MAP_CELLS * MAP_CELLS - 1) as usize], UNKNOWN);
    }
}
