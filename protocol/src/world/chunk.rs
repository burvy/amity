use avian3d::prelude::*;
use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: i32 = 32;
/// Edge length of one block in metres. Everything else in the voxel code counts in
/// blocks; convert to metres only through `block_corner` and `block_at`.
pub const BLOCK_SIZE: f32 = 0.5;
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

/// World position in metres of a block's minimum corner
pub fn block_corner(pos: IVec3) -> Vec3 {
    pos.as_vec3() * BLOCK_SIZE
}

/// The block containing a world point in metres
pub fn block_at(point: Vec3) -> IVec3 {
    (point / BLOCK_SIZE).floor().as_ivec3()
}

/// world block = chunk * CHUNK_SIZE + local
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ChunkPos(pub IVec3);

impl ChunkPos {
    /// world position of this chunk's (0, 0, 0) block corner
    pub fn origin(self) -> Vec3 {
        block_corner(self.0 * CHUNK_SIZE)
    }

    /// splits a world block coordinate into its chunk and the local position inside it.
    /// euclidean division so -1 lands in chunk -1 at local 31, not chunk 0 at local -1
    pub fn of_block(world: IVec3) -> (ChunkPos, IVec3) {
        let size = IVec3::splat(CHUNK_SIZE);
        (ChunkPos(world.div_euclid(size)), world.rem_euclid(size))
    }
}

/// Finds a chunk's entity by its position. Kept up to date by observers, so it
/// covers every chunk entity on both client and server.
#[derive(Resource, Default)]
pub struct ChunkIndex(HashMap<IVec3, Entity>);

impl ChunkIndex {
    pub fn get(&self, pos: ChunkPos) -> Option<Entity> {
        self.0.get(&pos.0).copied()
    }
}

pub fn index_chunk(add: On<Add, ChunkPos>, chunks: Query<&ChunkPos>, mut index: ResMut<ChunkIndex>) {
    if let Ok(pos) = chunks.get(add.entity) {
        index.0.insert(pos.0, add.entity);
    }
}

pub fn unindex_chunk(
    remove: On<Remove, ChunkPos>,
    chunks: Query<&ChunkPos>,
    mut index: ResMut<ChunkIndex>,
) {
    if let Ok(pos) = chunks.get(remove.entity) {
        index.0.remove(&pos.0);
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

    /// avian3d collider for voxels. None for an all-air chunk: avian can't build a
    /// bounding box around zero voxels
    pub fn collider(&self) -> Option<Collider> {
        let solid: Vec<IVec3> = self.solid().collect();
        (!solid.is_empty()).then(|| Collider::voxels(Vec3::splat(BLOCK_SIZE), &solid))
    }
}

/// Agreed-upon chunk entity for both client and server
/// The collider is added by `edit::sync_colliders`, since an all-air chunk has none.
/// Transform is explicit because nothing else would add it to a chunk without a collider.
pub fn chunk_body(pos: ChunkPos, chunk: Chunk) -> impl Bundle {
    (
        pos,
        Position(pos.origin()),
        Transform::from_translation(pos.origin()),
        RigidBody::Static,
        chunk,
    )
}
