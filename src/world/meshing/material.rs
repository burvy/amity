use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
};

use super::textures::ATLAS;

/// Shared by every chunk. Both use the block texture atlas
#[derive(Resource)]
pub struct ChunkMaterials {
    /// solid blocks
    pub solid: Handle<StandardMaterial>,
    /// fluids: blended with what's behind them by the texture's alpha
    pub fluid: Handle<StandardMaterial>,
}

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
    let atlas = images.add(atlas);
    cmds.insert_resource(ChunkMaterials {
        solid: mats.add(StandardMaterial {
            base_color_texture: Some(atlas.clone()),
            perceptual_roughness: 0.25,
            ..default()
        }),
        fluid: mats.add(StandardMaterial {
            base_color_texture: Some(atlas),
            perceptual_roughness: 0.1,
            // see-through, using the png's alpha
            alpha_mode: AlphaMode::Blend,
            // the surface is also seen from underneath, when underwater
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
    });
}
