use amity_protocol::sky::{
    astronomy::{moon_direction, moon_illumination, sun_direction},
    config::DAY_LENGTH_SECS,
    WorldClock,
};
use bevy::{
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    light::{
        atmosphere::{Falloff, PhaseFunction, ScatteringMedium, ScatteringTerm},
        Atmosphere, SunDisk,
    },
    pbr::AtmosphereSettings,
    prelude::*,
};

use super::config::*;
use crate::{
    player::definition::MainPlayerCamera,
    settings::definition::{Settings, ShadowQuality},
};

/// Our copy of the world's time, advanced every frame so the sun moves smoothly,
/// and pulled back in line whenever the server's `WorldClock` arrives
#[derive(Resource, Default)]
pub struct SkyTime {
    days: f64,
    synced: bool,
}

impl SkyTime {
    /// Days since 1 January of year 0 (see `WorldClock`), once the server's time
    /// has arrived
    pub fn days(&self) -> Option<f64> {
        self.synced.then_some(self.days)
    }
}

#[derive(Component)]
pub struct Sun;

#[derive(Component)]
pub struct Moon;

/// The planet's atmosphere, and the sun and moon as lights
pub fn spawn_sky(mut cmds: Commands, mut media: ResMut<Assets<ScatteringMedium>>) {
    // Earth's air: blue sky, red sunsets, and a haze near the ground that fades
    // distant land. Its transform is the planet's centre, placed by default one
    // earth radius below the origin, so the ground (sea level) is at y = 0
    let mut atmosphere = Atmosphere::earth(Handle::default());
    atmosphere.medium = media.add(air(atmosphere.outer_radius - atmosphere.inner_radius));
    cmds.spawn(atmosphere);
    cmds.spawn((
        Sun,
        DirectionalLight {
            illuminance: SUN_LUX,
            shadow_maps_enabled: true,
            ..default()
        },
        // the sun's visible disk, at its real size and brightness. Its shadow
        // cascades come from the Shadows setting (`settings/apply.rs`)
        SunDisk::EARTH,
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

/// Earth's air (gas, fine dust high up, ozone), plus haze: particles near the
/// ground, like the water and dust on a normal humid day. `depth` is how far the
/// atmosphere reaches up from the ground (metres).
///
/// The haze is set by how far you can see through it: the distance at which
/// something keeps 2% of its contrast is 3.912 divided by how much of the light
/// the air stops per metre (Koschmieder's law). It scatters most of that light
/// and absorbs the rest, and scatters blue a little more than red. Its density
/// falls off exponentially with height
fn air(depth: f32) -> ScatteringMedium {
    let mut medium = ScatteringMedium::earth(ATMOSPHERE_HEIGHT_STEPS, 256);
    let stopped = 3.912 / HAZE_VISIBILITY;
    let scattered = stopped * HAZE_SCATTER_SHARE;
    // relative to green light (550 nm): red (680 nm) scatters less, blue (440
    // nm) more
    let by_colour = Vec3::new(550.0f32 / 680.0, 1.0, 550.0 / 440.0).powf(HAZE_ANGSTROM);
    medium.terms.push(ScatteringTerm {
        absorption: Vec3::splat(stopped - scattered),
        scattering: by_colour * scattered,
        falloff: Falloff::Exponential {
            scale: HAZE_SCALE_HEIGHT / depth,
        },
        phase: PhaseFunction::Mie {
            asymmetry: HAZE_ASYMMETRY,
        },
    });
    medium.with_label("air_with_haze")
}

/// Turns the player's camera into a physically based one: the atmosphere drawn
/// as its sky, a fixed exposure, and a filmic response curve. The sky's
/// lighting of the world and bloom depend on the graphics settings, so
/// `settings/apply.rs` adds those
pub fn setup_camera(camera: On<Add, MainPlayerCamera>, mut cmds: Commands) {
    cmds.entity(camera.entity)
        .insert((
            AtmosphereSettings {
                aerial_view_lut_max_distance: AERIAL_PERSPECTIVE_DISTANCE,
                ..default()
            },
            Exposure { ev100: EXPOSURE_EV100 },
            Tonemapping::AgX,
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
    settings: Res<Settings>,
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
    let shadows = sun_up && settings.graphics.shadows != ShadowQuality::Off;
    if sun_light.shadow_maps_enabled != shadows {
        sun_light.shadow_maps_enabled = shadows;
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
