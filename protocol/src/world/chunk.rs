use avian3d::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: i32 = 32;
const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum Block {
    #[default]
    Air,
    Stone,
    Dirt,
    Grass,
}

impl Block {
    pub fn is_solid(self) -> bool {
        self != Block::Air
    }
}

/// world block = chunk * CHUNK_SIZE + local
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ChunkPos(pub IVec3);

impl ChunkPos {
    /// world position of this chunk's (0, 0, 0) block corner
    pub fn origin(self) -> Vec3 {
        (self.0 * CHUNK_SIZE).as_vec3()
    }
}

/// Server -> client: the full contents of one chunk
#[derive(Serialize, Deserialize, Clone)]
pub struct ChunkData {
    pub pos: ChunkPos,
    pub chunk: Chunk,
}

/// Ordered + reliable, so later block edits can never overtake the chunk they modify
pub struct WorldChannel;

#[derive(Component, Clone, Serialize, Deserialize)]
pub struct Chunk {
    blocks: Box<[Block]>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            blocks: vec![Block::Air; CHUNK_VOLUME].into_boxed_slice(),
        }
    }
}

impl Chunk {
    /// blocks are stored in a single dimensional "buffer". This calculation gets the
    /// index of the block at the x, y, z
    fn index(local: IVec3) -> usize {
        (local.x + local.z * CHUNK_SIZE + local.y * CHUNK_SIZE * CHUNK_SIZE) as usize
    }

    /// gets the block at the block at index x, y, z
    pub fn get(&self, local: IVec3) -> Block {
        self.blocks[Self::index(local)]
    }

    /// sets the block at the block at index x, y, z
    pub fn set(&mut self, local: IVec3, block: Block) {
        self.blocks[Self::index(local)] = block;
    }

    /// gets the position of all solid blocks and returns their
    /// coordinates as IVec3s.
    pub fn solid(&self) -> impl Iterator<Item = IVec3> + '_ {
        (0..CHUNK_VOLUME as i32)
            .filter(|&i| self.blocks[i as usize].is_solid())
            .map(|i| {
                IVec3::new(
                    i % CHUNK_SIZE,
                    i / (CHUNK_SIZE * CHUNK_SIZE),
                    (i / CHUNK_SIZE) % CHUNK_SIZE,
                )
            })
    }

    /// avian3d collider for voxels
    pub fn collider(&self) -> Collider {
        Collider::voxels(Vec3::ONE, &self.solid().collect::<Vec<_>>())
    }
}

/// Agreed-upon chunk entity for both client and server
pub fn chunk_body(pos: ChunkPos, chunk: Chunk) -> impl Bundle {
    (
        pos,
        Position(pos.origin()),
        RigidBody::Static,
        chunk.collider(),
        chunk,
    )
}
