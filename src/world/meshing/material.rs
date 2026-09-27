use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
};

use super::textures::{atlas, ATLAS};

/// Shared by every chunk. Both use the block texture atlas
#[derive(Resource)]
pub struct ChunkMaterials {
    /// solid blocks
    pub solid: Handle<StandardMaterial>,
    /// fluids: blended with what's behind them by the texture's alpha
    pub fluid: Handle<StandardMaterial>,
}

/// The block texture atlas, for UI icons: the image, and how it's cut into tiles
#[derive(Resource)]
pub struct BlockAtlas {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

pub fn init_chunk_material(
    mut cmds: Commands,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
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
    cmds.insert_resource(BlockAtlas {
        image: atlas.clone(),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(atlas::TILE_PIXELS),
            atlas::COLUMNS,
            atlas::ROWS,
            None,
            None,
        )),
    });
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
