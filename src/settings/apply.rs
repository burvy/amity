//! Putting the settings into effect: whenever they change (and once at start),
//! the camera, lights, window and UI are updated to match

use amity_protocol::world::messages::{EditChannel, SetViewDistance};
use bevy::{
    light::{
        AtmosphereEnvironmentMapLight, CascadeShadowConfigBuilder, DirectionalLightShadowMap,
    },
    anti_alias::{fxaa::Fxaa, smaa::Smaa},
    post_process::bloom::Bloom,
    prelude::*,
    window::{PresentMode, PrimaryWindow},
};
use lightyear::prelude::MessageSender;

use super::definition::{AntiAliasing, Settings};
use crate::{
    player::definition::MainPlayerCamera,
    sky::{config::*, logic::Sun},
};

/// Graphics: field of view, anti-aliasing, bloom, sky lighting, shadows, frame pacing
/// and UI size. Runs only when the settings changed
pub fn apply_graphics(
    settings: Res<Settings>,
    mut cmds: Commands,
    camera: Single<(Entity, &mut Projection), With<MainPlayerCamera>>,
    sun: Single<Entity, With<Sun>>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut ui_scale: ResMut<UiScale>,
) {
    let graphics = &settings.graphics;
    let (camera, mut projection) = camera.into_inner();
    if let Projection::Perspective(perspective) = &mut *projection {
        perspective.fov = graphics.fov.to_radians();
    }

    let mut camera = cmds.entity(camera);
    // the sky lighting the world: ambient light and reflections from its colour
    camera.insert(AtmosphereEnvironmentMapLight {
        size: UVec2::splat(graphics.sky_light.resolution()),
        ..default()
    });
    camera.insert(graphics.shadows.filtering());

    // one kind of anti-aliasing at a time (MSAA is the camera's sample count)
    camera.remove::<(Fxaa, Smaa)>();
    let msaa = if graphics.anti_aliasing == AntiAliasing::Msaa {
        Msaa::Sample4
    } else {
        Msaa::Off
    };
    camera.insert(msaa);
    match graphics.anti_aliasing {
        AntiAliasing::Fxaa => {
            camera.insert(Fxaa::default());
        }
        AntiAliasing::Smaa => {
            camera.insert(Smaa::default());
        }
        AntiAliasing::Off | AntiAliasing::Msaa => {}
    }
    if graphics.bloom {
        camera.insert(Bloom::NATURAL);
    } else {
        camera.remove::<Bloom>();
    }

    // shadows: cascades, reach and sharpness (whether they're on at all is also
    // up to the sun being up, see `sky::logic::move_lights`)
    if let Some((cascades, distance, resolution)) = graphics.shadows.cascades() {
        cmds.entity(*sun).insert(
            CascadeShadowConfigBuilder {
                num_cascades: cascades,
                first_cascade_far_bound: SHADOW_FIRST_CASCADE.min(distance / cascades as f32),
                maximum_distance: distance,
                ..default()
            }
            .build(),
        );
        shadow_map.size = resolution;
    }

    // VSync: in step with the screen. Otherwise as fast as allowed, capped by
    // `limit_frame_rate` if a number was chosen
    window.present_mode = match graphics.frame_limit.fps() {
        None if graphics.frame_limit == super::definition::FrameLimit::VSync => PresentMode::AutoVsync,
        _ => PresentMode::AutoNoVsync,
    };
    ui_scale.0 = graphics.ui_scale;
}

/// A frame rate cap: waits out the rest of each frame's time. Native only: the
/// browser paces the web version itself
#[cfg(not(target_family = "wasm"))]
pub fn limit_frame_rate(settings: Res<Settings>, mut last: Local<Option<std::time::Instant>>) {
    let Some(fps) = settings.graphics.frame_limit.fps() else {
        *last = None;
        return;
    };
    let frame = std::time::Duration::from_secs_f64(1.0 / fps);
    if let Some(previous) = *last {
        let elapsed = previous.elapsed();
        if elapsed < frame {
            std::thread::sleep(frame - elapsed);
        }
    }
    *last = Some(std::time::Instant::now());
}

/// Tells the server our render distance, whenever it changes (and once
/// connected)
pub fn send_render_distance(
    settings: Res<Settings>,
    sender: Option<Single<&mut MessageSender<SetViewDistance>>>,
    mut sent: Local<Option<u8>>,
) {
    let chunks = settings.graphics.render_distance;
    let Some(mut sender) = sender else {
        // not connected (yet, or any more): say it again once we are
        *sent = None;
        return;
    };
    if *sent != Some(chunks) {
        sender.send::<EditChannel>(SetViewDistance { chunks });
        *sent = Some(chunks);
    }
}
