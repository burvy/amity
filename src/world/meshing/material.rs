use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
};

use super::textures::ATLAS;

/// Shared by every chunk: the block texture atlas
#[derive(Resource)]
pub struct ChunkMaterial(pub Handle<StandardMaterial>);

pub fn init_chunk_material(
    mut cmds: Commands,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // nearest sampling keeps the pixel art crisp instead of blurring it
    let atlas = Image::from_buffer(
        ATLAS,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::nearest(),
        RenderAssetUsages::RENDER_WORLD,
    )
    .expect("the atlas is generated at build time, so it must be a valid png");
    cmds.insert_resource(ChunkMaterial(mats.add(StandardMaterial {
        base_color_texture: Some(images.add(atlas)),
        perceptual_roughness: 0.25,
        ..default()
    })));
}
