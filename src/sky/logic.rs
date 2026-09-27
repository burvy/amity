use amity_protocol::sky::{
    astronomy::{moon_direction, moon_illumination, sun_direction},
    config::DAY_LENGTH_SECS,
    WorldClock,
};
use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    light::{
        atmosphere::ScatteringMedium, Atmosphere, AtmosphereEnvironmentMapLight,
        CascadeShadowConfigBuilder, SunDisk,
    },
    pbr::AtmosphereSettings,
    post_process::{auto_exposure::AutoExposure, bloom::Bloom},
    prelude::*,
};

use super::config::*;
use crate::player::definition::MainPlayerCamera;

/// Our copy of the world's time, advanced every frame so the sun moves smoothly,
/// and pulled back in line whenever the server's `WorldClock` arrives
#[derive(Resource, Default)]
pub struct SkyTime {
    days: f64,
    synced: bool,
}

#[derive(Component)]
pub struct Sun;

#[derive(Component)]
pub struct Moon;

/// The planet's atmosphere, and the sun and moon as lights
pub fn spawn_sky(mut cmds: Commands, mut media: ResMut<Assets<ScatteringMedium>>) {
    // Earth's air: blue sky, red sunsets, haze over distance. Its transform is
    // the planet's centre, placed by default one earth radius below the origin
    cmds.spawn(Atmosphere::earth(media.add(ScatteringMedium::default())));
    cmds.spawn((
        Sun,
        DirectionalLight {
            illuminance: SUN_LUX,
            shadow_maps_enabled: true,
            ..default()
        },
        // the sun's visible disk, at its real size and brightness
        SunDisk::EARTH,
        CascadeShadowConfigBuilder {
            num_cascades: SHADOW_CASCADES,
            first_cascade_far_bound: SHADOW_FIRST_CASCADE,
            maximum_distance: SHADOW_DISTANCE,
            ..default()
        }
        .build(),
    ));
    cmds.spawn((
        Moon,
        DirectionalLight {
            illuminance: 0.0,
            shadow_maps_enabled: false,
            ..default()
        },
        // the moon looks about as big as the sun
        SunDisk::EARTH,
    ));
}

/// Turns the player's camera into a physically based one: the atmosphere drawn
/// as its sky and lighting the scene (ambient light and reflections from the
/// sky's colour), eyes that adapt to brightness, a filmic response curve, and a
/// glow around bright lights like the sun
pub fn setup_camera(camera: On<Add, MainPlayerCamera>, mut cmds: Commands) {
    cmds.entity(camera.entity).insert((
        AtmosphereSettings::default(),
        AtmosphereEnvironmentMapLight::default(),
        AutoExposure {
            range: EXPOSURE_RANGE,
            speed_brighten: EXPOSURE_BRIGHTEN_SPEED,
            speed_darken: EXPOSURE_DARKEN_SPEED,
            ..default()
        },
        Tonemapping::AgX,
        Bloom::NATURAL,
    ));
}

/// Advances our time, and snaps to the server's whenever it arrives. Its updates
/// come about once a second, so between them only small drift can build up
pub fn update_time(
    mut time: ResMut<SkyTime>,
    real: Res<Time>,
    clock: Option<Single<&WorldClock, Changed<WorldClock>>>,
) {
    time.days += real.delta_secs_f64() / DAY_LENGTH_SECS;
    if let Some(clock) = clock {
        time.days = clock.0;
        time.synced = true;
    }
}

/// Points the sun and moon lights from where they are in the sky, and sets the
/// moon's brightness from its phase
pub fn move_lights(
    time: Res<SkyTime>,
    mut sun: Single<(&mut Transform, &mut DirectionalLight), (With<Sun>, Without<Moon>)>,
    mut moon: Single<(&mut Transform, &mut DirectionalLight), (With<Moon>, Without<Sun>)>,
) {
    if !time.synced {
        return;
    }
    let (sun_transform, sun_light) = &mut *sun;
    let to_sun = sun_direction(time.days);
    // a directional light shines along its forward (-Z): away from the sun
    point_along(sun_transform, -to_sun);
    // below the horizon the planet blocks it (the atmosphere handles that), and
    // shadows cast from underneath the world would only cost time
    let sun_up = to_sun.y > -0.05;
    if sun_light.shadow_maps_enabled != sun_up {
        sun_light.shadow_maps_enabled = sun_up;
    }

    let (moon_transform, moon_light) = &mut *moon;
    point_along(moon_transform, -moon_direction(time.days));
    moon_light.illuminance = FULL_MOON_LUX * moon_illumination(time.days) as f32;
}

/// Turns `transform` to face `forward`
fn point_along(transform: &mut Transform, forward: Vec3) {
    // any "up" not parallel to forward works for a directional light
    let up = if forward.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    *transform = Transform::default().looking_to(forward, up);
}
