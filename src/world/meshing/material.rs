//! The material every chunk is drawn with: bevy's standard lit material, plus
//! a small shader (`block.wgsl`) that repeats each block's texture across the
//! mesher's merged rectangles

use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

use super::textures::{atlas, tile_inset, tile_size, ATLAS};

/// The block shader, embedded (see `MeshingPlugin`)
const SHADER: &str = "embedded://amity/world/meshing/block.wgsl";

/// The standard material, drawn through the block shader
pub type BlockMaterial = ExtendedMaterial<StandardMaterial, BlockTiling>;

/// What the block shader needs to find a block's pixels in the atlas
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct BlockTiling {
    /// xy: a tile's size in atlas UV. z: how far in from a tile's edges to
    /// sample, as a share of the tile. w: unused
    #[uniform(100)]
    tiling: Vec4,
}

impl Default for BlockTiling {
    fn default() -> Self {
        Self {
            tiling: tile_size().extend(tile_inset()).extend(0.0),
        }
    }
}

impl MaterialExtension for BlockTiling {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}

/// Shared by every chunk. Both use the block texture atlas
#[derive(Resource)]
pub struct ChunkMaterials {
    /// solid blocks
    pub solid: Handle<BlockMaterial>,
    /// fluids: blended with what's behind them by the texture's alpha
    pub fluid: Handle<BlockMaterial>,
}

/// The block texture atlas, for UI icons: the image, and how it's cut into tiles
#[derive(Resource)]
pub struct BlockAtlas {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

pub fn init_chunk_material(
    mut cmds: Commands,
    mut mats: ResMut<Assets<BlockMaterial>>,
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
    let block = |base: StandardMaterial| BlockMaterial {
        base,
        extension: BlockTiling::default(),
    };
    cmds.insert_resource(ChunkMaterials {
        solid: mats.add(block(StandardMaterial {
            base_color_texture: Some(atlas.clone()),
            // rough and matte, like real stone, soil and wood: little shine from
            // the sun, and only faint reflections of the sky
            perceptual_roughness: 0.9,
            ..default()
        })),
        fluid: mats.add(block(StandardMaterial {
            base_color_texture: Some(atlas),
            perceptual_roughness: 0.1,
            // see-through, using the png's alpha
            alpha_mode: AlphaMode::Blend,
            // the surface is also seen from underneath, when underwater
            double_sided: true,
            cull_mode: None,
            ..default()
        })),
    });
}
