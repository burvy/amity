//! Keeping the horizon map up to date: redone in the background around the
//! player as chunks arrive, change or leave, and when the player moves far

use amity_protocol::world::{
    chunk::{ChunkPos, ChunkSky},
    config::{BLOCK_SIZE, CHUNK_SIZE},
};
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    tasks::{futures::check_ready, AsyncComputeTaskPool, Task},
};

use super::{
    compute::{horizon_texture, Snapshot, DIRECTIONS},
    config::*,
};
use crate::{
    player::definition::MainPlayerCamera,
    settings::definition::Settings,
    world::meshing::material::{BlockMaterial, ChunkMaterials},
};

/// The horizon map's texture, which the block material reads
#[derive(Resource)]
pub struct HorizonMap {
    pub image: Handle<Image>,
}

/// The map being redone, and when and where it last was
#[derive(Resource, Default)]
pub struct HorizonUpdate {
    task: Option<Task<Vec<u8>>>,
    /// something changed since the last update started
    stale: bool,
    /// the cell the last update was centred on
    centre: Option<IVec2>,
    /// seconds (app time) the last update started
    started: f32,
}

/// An empty map: 2 layers (directions 0-3 and 4-7) of MAP_CELLS squared.
/// Repeating at its edges, which is what wraps world positions onto it, and
/// blended between cells so shadow edges are smooth
pub fn init_horizon_map(mut cmds: Commands, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d {
        width: MAP_CELLS as u32,
        height: MAP_CELLS as u32,
        depth_or_array_layers: (DIRECTIONS / 4) as u32,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    cmds.insert_resource(HorizonMap {
        image: images.add(image),
    });
}

/// The cell a world position is in
fn cell_of(position: Vec3) -> IVec2 {
    let cell_metres = CELL_BLOCKS as f32 * BLOCK_SIZE;
    (position.xz() / cell_metres).floor().as_ivec2()
}

/// Starts an update when chunks changed (at most every UPDATE_SECS) or the
/// player moved RECENTRE_CELLS from the map's centre, unless one is running
pub fn start_update(
    mut update: ResMut<HorizonUpdate>,
    time: Res<Time>,
    changed: Query<(), Changed<ChunkSky>>,
    mut removed: RemovedComponents<ChunkSky>,
    skies: Query<(&ChunkPos, &ChunkSky)>,
    camera: Single<&GlobalTransform, With<MainPlayerCamera>>,
) {
    if !changed.is_empty() || removed.read().next().is_some() {
        update.stale = true;
    }
    let centre = cell_of(camera.translation());
    let moved = update
        .centre
        .is_none_or(|last| (centre - last).abs().max_element() >= RECENTRE_CELLS);
    let now = time.elapsed_secs();
    let due = update.stale && now - update.started >= UPDATE_SECS;
    if update.task.is_some() || !(moved || due) {
        return;
    }

    // copies of the sky of the chunks within the map (most chunks' sky is a
    // single number, so this is cheap)
    let reach = (MAP_CELLS * CELL_BLOCKS) / CHUNK_SIZE / 2 + 1;
    let centre_chunk = (centre * CELL_BLOCKS).div_euclid(IVec2::splat(CHUNK_SIZE));
    let snapshot = Snapshot {
        skies: skies
            .iter()
            .filter(|(pos, _)| (pos.0.xz() - centre_chunk).abs().max_element() <= reach)
            .map(|(pos, sky)| (pos.0, sky.clone()))
            .collect(),
        corner: centre - IVec2::splat(MAP_CELLS / 2),
    };
    update.task = Some(AsyncComputeTaskPool::get().spawn(async move { horizon_texture(&snapshot) }));
    update.stale = false;
    update.centre = Some(centre);
    update.started = now;
}

/// Puts a finished map in the texture. The block materials are marked changed
/// too, so they pick up the new texture
pub fn finish_update(
    mut update: ResMut<HorizonUpdate>,
    map: Res<HorizonMap>,
    mut images: ResMut<Assets<Image>>,
    chunk_materials: Option<Res<ChunkMaterials>>,
    mut materials: ResMut<Assets<BlockMaterial>>,
) {
    let Some(task) = &mut update.task else {
        return;
    };
    let Some(data) = check_ready(task) else {
        return;
    };
    update.task = None;
    if let Some(mut image) = images.get_mut(&map.image) {
        image.data = Some(data);
    }
    if let Some(chunk_materials) = chunk_materials {
        materials.get_mut(&chunk_materials.solid);
        materials.get_mut(&chunk_materials.fluid);
    }
}

/// Tells the block shader how far the shadow map reaches (the Shadows
/// setting), so its shadows fade into the horizon's before they end
pub fn shadow_reach(
    settings: Res<Settings>,
    chunk_materials: Option<Res<ChunkMaterials>>,
    mut materials: ResMut<Assets<BlockMaterial>>,
) {
    let Some(chunk_materials) = chunk_materials else {
        return;
    };
    let reach = settings
        .graphics
        .shadows
        .cascades()
        .map_or(0.0, |(_, distance, _)| distance);
    for handle in [&chunk_materials.solid, &chunk_materials.fluid] {
        if let Some(mut material) = materials.get_mut(handle) {
            material.extension.set_shadow_reach(reach);
        }
    }
}
