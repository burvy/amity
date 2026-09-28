//! Render scale: drawing the world at fewer pixels than the screen has, then
//! stretching it to fit. On a weak GPU or a very sharp screen this is the
//! biggest single saving (75% is about half the pixels). The UI is drawn on top
//! at full resolution, so text stays sharp.
//!
//! How: below 100%, the world camera draws into an image instead of the window,
//! and a full-screen UI image, underneath all other UI, shows it stretched. The
//! UI has its own camera, drawn over the world. At 100% the world camera draws
//! straight to the window, with no image and no extra copy.

use bevy::{
    camera::{CameraOutputMode, RenderTarget},
    image::ImageSampler,
    prelude::*,
    render::render_resource::{BlendState, TextureFormat},
    window::{PrimaryWindow, WindowRef},
};

use super::definition::Settings;
use crate::player::definition::MainPlayerCamera;

/// The camera that draws all UI, over the world
#[derive(Component)]
pub struct UiCamera;

/// The full-screen image showing the world, below 100% render scale
#[derive(Component)]
pub struct ScaledView;

pub fn spawn_ui_camera(mut cmds: Commands) {
    cmds.spawn((
        UiCamera,
        Camera2d,
        Camera {
            // after the world camera
            order: 1,
            // The UI draws into its own image first (the world camera's is HDR,
            // this one's isn't, so they can't share one). Starting that image
            // transparent and blending it over the window keeps the world
            // showing through; written as-is, it would paint over it. (Drawing
            // onto transparent leaves colours already multiplied by their
            // opacity, hence "premultiplied")
            clear_color: ClearColorConfig::Custom(Color::NONE),
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            ..default()
        },
        Msaa::Off,
        IsDefaultUiCamera,
    ));
    cmds.spawn((
        ScaledView,
        ImageNode::default(),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        // under every other UI
        GlobalZIndex(i32::MIN),
        Visibility::Hidden,
    ));
}

/// Points the world camera at the window (100%) or at an image of the scaled
/// size, made again whenever the window or the setting changes size
pub fn apply_render_scale(
    settings: Res<Settings>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(Entity, &RenderTarget), With<MainPlayerCamera>>,
    view: Single<(&mut ImageNode, &mut Visibility), With<ScaledView>>,
    mut images: ResMut<Assets<Image>>,
    mut cmds: Commands,
) {
    let scale = settings.graphics.render_scale;
    let (camera, target) = camera.into_inner();
    let (mut view_image, mut visibility) = view.into_inner();

    if scale >= 1.0 {
        if !matches!(target, RenderTarget::Window(_)) {
            cmds.entity(camera)
                .insert(RenderTarget::Window(WindowRef::Primary));
        }
        visibility.set_if_neq(Visibility::Hidden);
        return;
    }

    let full = UVec2::new(window.physical_width(), window.physical_height());
    let size = (full.as_vec2() * scale).round().as_uvec2().max(UVec2::ONE);
    let current = match target {
        RenderTarget::Image(image) => images.get(&image.handle).map(|image| image.size()),
        _ => None,
    };
    if current != Some(size) {
        let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
        // smooth when stretched (the block textures' pixel-sharp sampling
        // would make it blocky)
        image.sampler = ImageSampler::linear();
        let handle = images.add(image);
        cmds.entity(camera)
            .insert(RenderTarget::Image(handle.clone().into()));
        // the old image is freed once nothing holds it
        view_image.image = handle;
    }
    visibility.set_if_neq(Visibility::Inherited);
}
