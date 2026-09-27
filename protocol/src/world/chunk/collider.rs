//! Chunk colliders, kept matching the blocks as they change

use avian3d::{parry::shape::SharedShape, prelude::*};
use bevy::prelude::*;

use super::Chunk;
use crate::world::{
    blocks::BlockSet,
    config::{BLOCK_SIZE, CHUNK_SIZE},
};

/// The collider for a chunk's blocks, anchored at its origin corner. None for an
/// all-air chunk: avian can't build a bounding box around zero voxels.
/// A chunk of one solid block (deep stone) gets a single box instead of 32^3
/// voxels, which is far cheaper to build and keep; its first edit swaps in voxels.
pub fn chunk_collider(chunk: &Chunk) -> Option<Collider> {
    match chunk {
        Chunk::Uniform(block) if !block.is_solid() => None,
        Chunk::Uniform(_) => {
            let size = CHUNK_SIZE as f32 * BLOCK_SIZE;
            Some(Collider::compound(vec![(
                Vec3::splat(size / 2.0),
                Quat::IDENTITY,
                Collider::cuboid(size, size, size),
            )]))
        }
        Chunk::Paletted(_) => {
            let solid: Vec<IVec3> = chunk.solid().collect();
            (!solid.is_empty()).then(|| Collider::voxels(Vec3::splat(BLOCK_SIZE), &solid))
        }
    }
}

/// Builds a newly spawned chunk's collider from all of its blocks
pub fn init_colliders(mut cmds: Commands, chunks: Query<(Entity, &Chunk), Added<Chunk>>) {
    for (entity, chunk) in &chunks {
        if let Some(collider) = chunk_collider(chunk) {
            cmds.entity(entity).insert(collider);
        }
    }
}

/// Applies block changes to chunk colliders one voxel at a time. A chunk that
/// becomes all air loses its collider; an empty chunk gets one on its first block.
pub fn update_colliders(
    mut cmds: Commands,
    mut changes: MessageReader<BlockSet>,
    mut chunks: Query<(&Chunk, Option<&mut Collider>)>,
) {
    for change in changes.read() {
        let Ok((chunk, collider)) = chunks.get_mut(change.chunk) else {
            continue;
        };
        let Some(mut collider) = collider else {
            if let Some(collider) = chunk_collider(chunk) {
                cmds.entity(change.chunk).insert(collider);
            }
            continue;
        };
        if !change.block.is_solid() && chunk.is_empty() {
            cmds.entity(change.chunk).remove::<Collider>();
            continue;
        }
        // a solid chunk's single box: rebuild it as voxels, which then take edits
        let Some(voxels) = collider.shape().as_voxels() else {
            if let Some(collider) = chunk_collider(chunk) {
                cmds.entity(change.chunk).insert(collider);
            }
            continue;
        };
        // edit a copy and hand it back through `set_shape`: avian keeps a second,
        // scaled copy of the shape for physics, and only `set_shape` refreshes it
        let mut voxels = voxels.clone();
        voxels.set_voxel(change.local, change.block.is_solid());
        collider.set_shape(SharedShape::new(voxels));
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::{RunSystemOnce, SystemState};

    use super::*;
    use crate::world::{
        block::Block,
        blocks::Blocks,
        chunk::{pos, ChunkIndex, ChunkPos},
    };

    fn world_with(chunk: Chunk) -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<ChunkIndex>();
        world.init_resource::<Messages<BlockSet>>();
        world.add_observer(pos::index_chunk);
        let collider = chunk_collider(&chunk).unwrap();
        let entity = world.spawn((ChunkPos(IVec3::ZERO), chunk, collider)).id();
        (world, entity)
    }

    fn set_block(world: &mut World, pos: IVec3, block: Block) {
        let mut state = SystemState::<Blocks>::new(world);
        state.get_mut(world).unwrap().set(pos, block);
        state.apply(world);
        world.run_system_once(update_colliders).unwrap();
        world.flush();
    }

    #[test]
    fn edits_update_both_copies_of_the_collider() {
        let mut start = Chunk::default();
        start.set(IVec3::ZERO, Block::Stone);
        let (mut world, entity) = world_with(start);
        set_block(&mut world, IVec3::X, Block::Stone);

        let collider = world.get::<Collider>(entity).unwrap();
        for shape in [collider.shape(), collider.shape_scaled()] {
            let voxels = shape.as_voxels().unwrap();
            assert!(!voxels.voxel_state(IVec3::X).unwrap().is_empty());
        }
    }

    #[test]
    fn editing_a_solid_chunk_swaps_its_box_for_voxels() {
        let (mut world, entity) = world_with(Chunk::Uniform(Block::Stone));
        assert!(world.get::<Collider>(entity).unwrap().shape().as_voxels().is_none());
        set_block(&mut world, IVec3::ZERO, Block::Air);

        let collider = world.get::<Collider>(entity).unwrap();
        let voxels = collider.shape().as_voxels().unwrap();
        assert!(voxels.voxel_state(IVec3::ZERO).unwrap().is_empty());
        assert!(!voxels.voxel_state(IVec3::X).unwrap().is_empty());
    }
}
