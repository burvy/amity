//! Sky light, worked out for each chunk as it's meshed.
//!
//! Every block with open sky above it (see protocol `chunk/sky.rs`) gets the
//! full level, `SKY_LIGHT_MAX`. From there light spreads into every block that
//! isn't solid (air and water), losing one level per block, the way Minecraft
//! floods it: so it reaches `SKY_LIGHT_MAX` blocks under an overhang, into a
//! cave mouth or down into a lake, and deeper is pitch black.
//!
//! It only lights what the sky lights: the shader turns down the sky's
//! ambient light and reflections by it, never the sun (its shadows handle that)
//! or lamps and flashlights.
//!
//! The chunk's light depends on blocks up to `SKY_LIGHT_MAX` beyond its edges,
//! so it's worked out over the chunk plus that much all round, from the
//! neighbouring chunks' copies. No light is stored between meshes: a chunk's
//! light is simply worked out again whenever it's remeshed.

use std::collections::VecDeque;

use amity_protocol::world::config::CHUNK_SIZE;
use bevy::prelude::*;

use super::{
    config::SKY_LIGHT_MAX,
    padded::{Neighbourhood, PaddedChunk, PADDED_VOLUME},
};

const DIRECTIONS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

/// Sky light for the chunk and its one-block border, laid out like
/// `PaddedChunk`
pub fn sky_light(area: &Neighbourhood) -> Box<[u8]> {
    let mut light = vec![0; PADDED_VOLUME];

    // Shortcut: every block that isn't solid is out under the sky (the usual
    // case above ground). Then there's nowhere for light to spread to
    let mut shaded = false;
    for_each_cell(-1, CHUNK_SIZE + 2, |p| {
        if area.block(p).is_solid() {
            return;
        }
        if area.open(p) {
            light[PaddedChunk::index(p)] = SKY_LIGHT_MAX;
        } else {
            shaded = true;
        }
    });
    if !shaded {
        return light.into_boxed_slice();
    }

    // The region light can come from: the chunk plus SKY_LIGHT_MAX all round
    let reach = SKY_LIGHT_MAX as i32;
    let low = -reach;
    let size = CHUNK_SIZE + 2 * reach;
    let index = |p: IVec3| {
        let p = p - IVec3::splat(low);
        (p.x + p.z * size + p.y * size * size) as usize
    };
    let inside = |p: IVec3| p.cmpge(IVec3::splat(low)).all() && p.cmplt(IVec3::splat(low + size)).all();

    let volume = (size * size * size) as usize;
    let mut solid = vec![false; volume];
    let mut level = vec![0u8; volume];
    for_each_cell(low, size, |p| {
        if area.block(p).is_solid() {
            solid[index(p)] = true;
        } else if area.open(p) {
            level[index(p)] = SKY_LIGHT_MAX;
        }
    });

    // Flood outwards from the open blocks next to shaded ones, one level lost
    // per block, until it runs out
    let mut queue = VecDeque::new();
    for_each_cell(low, size, |p| {
        if level[index(p)] != SKY_LIGHT_MAX {
            return;
        }
        let shaded_beside = DIRECTIONS.iter().any(|&d| {
            let n = p + d;
            inside(n) && !solid[index(n)] && level[index(n)] < SKY_LIGHT_MAX
        });
        if shaded_beside {
            queue.push_back(p);
        }
    });
    while let Some(p) = queue.pop_front() {
        let next = level[index(p)].saturating_sub(1);
        if next == 0 {
            continue;
        }
        for d in DIRECTIONS {
            let n = p + d;
            if inside(n) && !solid[index(n)] && level[index(n)] < next {
                level[index(n)] = next;
                queue.push_back(n);
            }
        }
    }

    for_each_cell(-1, CHUNK_SIZE + 2, |p| {
        light[PaddedChunk::index(p)] = level[index(p)];
    });
    light.into_boxed_slice()
}

/// Calls `f` for every block of the cube from `low` to `low + size - 1` on each axis
fn for_each_cell(low: i32, size: i32, mut f: impl FnMut(IVec3)) {
    for y in low..low + size {
        for z in low..low + size {
            for x in low..low + size {
                f(IVec3::new(x, y, z));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use amity_protocol::world::{block::Block, chunk::{Chunk, ChunkSky}};

    use super::*;

    /// A chunk whose sky opens at local height `open` everywhere, with the
    /// chunks around it empty and open
    fn area(chunk: &Chunk, open: u8) -> Neighbourhood {
        let sky = ChunkSky::Uniform(open);
        let air = Chunk::default();
        let open_sky = ChunkSky::Uniform(0);
        Neighbourhood::gather(|offset| {
            if offset == IVec3::ZERO {
                Some((chunk, Some(&sky)))
            } else {
                Some((&air, Some(&open_sky)))
            }
        })
    }

    #[test]
    fn open_sky_is_full_light() {
        let light = sky_light(&area(&Chunk::default(), 0));
        assert!(light.iter().all(|&level| level == SKY_LIGHT_MAX));
    }

    #[test]
    fn light_fades_under_a_roof() {
        // a roof over everything at y = 20: open from 21 up. Below it, light
        // comes in sideways from the neighbouring chunks (open everywhere)
        let mut chunk = Chunk::default();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                chunk.set(IVec3::new(x, 20, z), Block::STONE);
            }
        }
        let light = sky_light(&area(&chunk, 21));
        let at = |x, y, z| light[PaddedChunk::index(IVec3::new(x, y, z))];
        assert_eq!(at(5, 25, 5), SKY_LIGHT_MAX);
        // just under the roof, far from the open chunk below: one block in
        // from the open side, and further in
        assert_eq!(at(0, 19, 16), SKY_LIGHT_MAX - 1);
        assert_eq!(at(5, 19, 16), SKY_LIGHT_MAX - 6);
        // the middle is more than SKY_LIGHT_MAX from every open side: dark
        assert_eq!(at(16, 19, 16), 0);
        // the roof itself is solid
        assert_eq!(at(5, 20, 5), 0);
    }
}
