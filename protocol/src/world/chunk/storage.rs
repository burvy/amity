//! How a chunk's blocks are stored, like Minecraft's paletted containers.
//! Most chunks are a single block (open sky, deep stone), and the rest hold only
//! a few block types, so a full byte per block wastes most of it:
//!
//! - `Uniform`: the whole chunk is one block. 1 byte instead of 32 KiB.
//! - `Paletted`: the list of block types present (the palette), plus one palette
//!   index per block, packed into as few bits as the palette needs:
//!   2 types -> 1 bit, 3-4 -> 2 bits, 5-16 -> 4 bits, 17-256 -> 8 bits,
//!   more -> 16 bits.
//!   Air, grass, dirt and stone fit in 2 bits: 8 KiB instead of 32 KiB.
//!
//! This same form is what goes over the network, and later into save files.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::world::{block::Block, config::CHUNK_SIZE};

/// Blocks in one chunk
const VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

#[derive(Component, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Chunk {
    Uniform(Block),
    Paletted(Paletted),
}

impl Default for Chunk {
    fn default() -> Self {
        Chunk::Uniform(Block::AIR)
    }
}

impl Chunk {
    /// Blocks are stored in one flat list: x changes fastest, then z, then y.
    /// So a row along x is CHUNK_SIZE neighbours in the list
    fn index(local: IVec3) -> usize {
        (local.x + local.z * CHUNK_SIZE + local.y * CHUNK_SIZE * CHUNK_SIZE) as usize
    }

    /// The reverse of `index`
    fn local(index: usize) -> IVec3 {
        let i = index as i32;
        IVec3::new(
            i % CHUNK_SIZE,
            i / (CHUNK_SIZE * CHUNK_SIZE),
            (i / CHUNK_SIZE) % CHUNK_SIZE,
        )
    }

    fn get_index(&self, index: usize) -> Block {
        match self {
            Chunk::Uniform(block) => *block,
            Chunk::Paletted(paletted) => paletted.get(index),
        }
    }

    /// The block at local position x, y, z (each 0..CHUNK_SIZE)
    pub fn get(&self, local: IVec3) -> Block {
        self.get_index(Self::index(local))
    }

    /// Sets the block at local position x, y, z (each 0..CHUNK_SIZE)
    pub fn set(&mut self, local: IVec3, block: Block) {
        let index = Self::index(local);
        match self {
            Chunk::Uniform(current) if *current == block => {}
            Chunk::Uniform(current) => {
                // the first different block: every block so far is the old one
                let mut paletted = Paletted::filled(*current);
                paletted.set(index, block);
                *self = Chunk::Paletted(paletted);
            }
            Chunk::Paletted(paletted) => paletted.set(index, block),
        }
    }

    /// Writes the CHUNK_SIZE blocks along x at height `y`, depth `z` into `out`
    pub fn read_row(&self, y: i32, z: i32, out: &mut [Block]) {
        let start = Self::index(IVec3::new(0, y, z));
        match self {
            Chunk::Uniform(block) => out.fill(*block),
            Chunk::Paletted(paletted) => {
                for (x, slot) in out.iter_mut().enumerate() {
                    *slot = paletted.get(start + x);
                }
            }
        }
    }

    /// True if there are no solid blocks
    pub fn is_empty(&self) -> bool {
        match self {
            Chunk::Uniform(block) => !block.is_solid(),
            Chunk::Paletted(paletted) => {
                // cheap check first: no solid type in the palette means no solid block
                !paletted.palette.iter().any(|block| block.is_solid())
                    || (0..VOLUME).all(|i| !paletted.get(i).is_solid())
            }
        }
    }

    /// Local positions of every solid block
    pub fn solid(&self) -> impl Iterator<Item = IVec3> + '_ {
        (0..VOLUME)
            .filter(|&i| self.get_index(i).is_solid())
            .map(Self::local)
    }

    /// A copy in the smallest form: edits only ever add palette entries, so blocks
    /// that were since removed are dropped here, and a chunk left with one block
    /// type becomes `Uniform`. Used before sending (and later saving).
    pub fn compacted(&self) -> Chunk {
        let Chunk::Paletted(paletted) = self else {
            return self.clone();
        };
        let mut used = vec![false; paletted.palette.len()];
        for i in 0..VOLUME {
            used[paletted.entry(i)] = true;
        }
        let kept: Vec<Block> = paletted
            .palette
            .iter()
            .zip(&used)
            .filter(|(_, used)| **used)
            .map(|(block, _)| *block)
            .collect();
        if let [only] = kept[..] {
            return Chunk::Uniform(only);
        }

        // old palette entry -> new palette entry
        let mut remap = vec![0; paletted.palette.len()];
        for (new, block) in kept.iter().enumerate() {
            let old = paletted.palette.iter().position(|b| b == block).unwrap();
            remap[old] = new;
        }
        let mut compact = Paletted::empty(kept);
        for i in 0..VOLUME {
            compact.write(i, remap[paletted.entry(i)]);
        }
        Chunk::Paletted(compact)
    }

    /// Roughly how many bytes this chunk takes to send
    pub fn byte_size(&self) -> usize {
        match self {
            Chunk::Uniform(_) => 1,
            Chunk::Paletted(paletted) => paletted.palette.len() + paletted.data.len(),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Paletted {
    /// Each block type in the chunk, once. Blocks store an index into this
    palette: Vec<Block>,
    /// Bits per palette index: 1, 2, 4, 8 or 16. Up to 8 they divide a byte, so
    /// an index never straddles two bytes; 16 is exactly two bytes
    bits: u8,
    /// VOLUME palette indices, `8 / bits` to a byte, lowest bits first
    data: Box<[u8]>,
}

impl Paletted {
    /// Every block `block`, ready for a second type to be set
    fn filled(block: Block) -> Self {
        Self::empty(vec![block])
    }

    /// All indices 0 (the first palette entry), sized for the whole palette
    fn empty(palette: Vec<Block>) -> Self {
        let bits = bits_for(palette.len());
        Self {
            palette,
            bits,
            data: vec![0; VOLUME * bits as usize / 8].into_boxed_slice(),
        }
    }

    fn get(&self, index: usize) -> Block {
        self.palette[self.entry(index)]
    }

    fn set(&mut self, index: usize, block: Block) {
        let entry = match self.palette.iter().position(|&b| b == block) {
            Some(entry) => entry,
            None => {
                self.palette.push(block);
                // the new entry doesn't fit in the current width: widen every index
                let bits = bits_for(self.palette.len());
                if bits != self.bits {
                    self.repack(bits);
                }
                self.palette.len() - 1
            }
        };
        self.write(index, entry);
    }

    /// The palette index stored for block `index`
    fn entry(&self, index: usize) -> usize {
        read_entry(&self.data, self.bits, index)
    }

    fn write(&mut self, index: usize, entry: usize) {
        if self.bits == 16 {
            let at = index * 2;
            self.data[at..at + 2].copy_from_slice(&(entry as u16).to_le_bytes());
            return;
        }
        let bit = index * self.bits as usize;
        let shift = bit % 8;
        let mask = (((1u16 << self.bits) - 1) as u8) << shift;
        let byte = &mut self.data[bit / 8];
        *byte = (*byte & !mask) | (((entry as u8) << shift) & mask);
    }

    /// Re-stores every index at a new width
    fn repack(&mut self, bits: u8) {
        let old_bits = std::mem::replace(&mut self.bits, bits);
        let old_data = std::mem::replace(
            &mut self.data,
            vec![0; VOLUME * bits as usize / 8].into_boxed_slice(),
        );
        for i in 0..VOLUME {
            self.write(i, read_entry(&old_data, old_bits, i));
        }
    }
}

/// Reads palette index `index` from indices packed `bits` wide
fn read_entry(data: &[u8], bits: u8, index: usize) -> usize {
    if bits == 16 {
        return u16::from_le_bytes([data[index * 2], data[index * 2 + 1]]) as usize;
    }
    let bit = index * bits as usize;
    let mask = (1 << bits) - 1;
    (data[bit / 8] as usize >> (bit % 8)) & mask
}

/// Fewest bits (1, 2, 4, 8 or 16) that can index a palette of `len` entries
fn bits_for(len: usize) -> u8 {
    match len {
        0..=2 => 1,
        3..=4 => 2,
        5..=16 => 4,
        17..=256 => 8,
        _ => 16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPES: [Block; 4] = [Block::AIR, Block::STONE, Block::DIRT, Block::GRASS];

    /// A block for every position that mixes all types irregularly
    fn pattern(local: IVec3) -> Block {
        TYPES[((local.x * 7 + local.y * 3 + local.z * 5) % 4) as usize]
    }

    fn every_local() -> impl Iterator<Item = IVec3> {
        (0..VOLUME).map(Chunk::local)
    }

    #[test]
    fn index_and_local_are_inverses() {
        for i in [0, 1, 31, 32, 1023, 1024, VOLUME - 1] {
            assert_eq!(Chunk::index(Chunk::local(i)), i);
        }
    }

    #[test]
    fn stores_every_block_through_palette_growth() {
        let mut chunk = Chunk::default();
        // 1 type -> uniform, 2 -> 1 bit, 3..4 -> 2 bits: every width change happens
        for local in every_local() {
            chunk.set(local, pattern(local));
        }
        for local in every_local() {
            assert_eq!(chunk.get(local), pattern(local), "{local}");
        }
        let Chunk::Paletted(paletted) = &chunk else {
            panic!("four block types can't be uniform");
        };
        assert_eq!(paletted.bits, 2);
        assert_eq!(chunk.byte_size(), 4 + VOLUME / 4);
    }

    #[test]
    fn compacting_drops_unused_types() {
        let mut chunk = Chunk::Uniform(Block::STONE);
        chunk.set(IVec3::ONE, Block::DIRT);
        chunk.set(IVec3::ZERO, Block::GRASS);
        chunk.set(IVec3::ONE, Block::STONE);

        let compact = chunk.compacted();
        let Chunk::Paletted(paletted) = &compact else {
            panic!("stone and grass remain");
        };
        assert_eq!(paletted.bits, 1);
        for local in every_local() {
            assert_eq!(compact.get(local), chunk.get(local));
        }

        chunk.set(IVec3::ZERO, Block::STONE);
        assert_eq!(chunk.compacted(), Chunk::Uniform(Block::STONE));
    }

    #[test]
    fn rows_match_single_reads() {
        let mut chunk = Chunk::default();
        for local in every_local() {
            chunk.set(local, pattern(local));
        }
        let mut row = [Block::AIR; CHUNK_SIZE as usize];
        chunk.read_row(5, 9, &mut row);
        for (x, block) in row.iter().enumerate() {
            assert_eq!(*block, chunk.get(IVec3::new(x as i32, 5, 9)));
        }
    }

    #[test]
    fn sixteen_bit_indices_round_trip() {
        // more than 256 palette entries (repeats are fine for testing the packing)
        let mut paletted = Paletted::empty(vec![Block::AIR; 300]);
        assert_eq!(paletted.bits, 16);
        for (index, entry) in [(0, 299), (1, 257), (VOLUME - 1, 1)] {
            paletted.write(index, entry);
            assert_eq!(paletted.entry(index), entry);
        }
        assert_eq!(paletted.entry(2), 0);
    }

    #[test]
    fn emptiness_ignores_leftover_palette_entries() {
        let mut chunk = Chunk::default();
        assert!(chunk.is_empty());
        chunk.set(IVec3::ONE, Block::STONE);
        assert!(!chunk.is_empty());
        chunk.set(IVec3::ONE, Block::AIR);
        assert!(chunk.is_empty());
    }
}
