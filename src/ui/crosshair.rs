//! The crosshair: four lines around a centre dot, inverting whatever is behind
//! them so they show on snow, sky and caves alike. The lines move apart in
//! proportion to how fast the player moves, and close in when they're still;
//! fast enough and they leave the screen. The spread is only for show:
//! breaking and placing always go exactly where the dot is.
//!
//! How the inverting works: the crosshair has its own camera, drawn last,
//! whose picture (white lines on nothing) is blended onto the finished frame
//! as "one minus what's there". The camera only covers a box around the lines,
//! grown and shrunk with them, so the blend touches a few thousand pixels
//! rather than the whole screen.

use avian3d::prelude::LinearVelocity;
use bevy::{
    camera::{CameraOutputMode, Viewport},
    prelude::*,
    render::render_resource::{BlendComponent, BlendFactor, BlendOperation, BlendState},
    window::PrimaryWindow,
};
use lightyear::prelude::Controlled;

use super::config::{
    CROSSHAIR_DOT, CROSSHAIR_GAP, CROSSHAIR_LINE_LENGTH, CROSSHAIR_LINE_THICKNESS,
    CROSSHAIR_PADDING, CROSSHAIR_SETTLE, CROSSHAIR_SPREAD_PER_SPEED,
};
use crate::player::definition::PlayerInfo;

/// Blends the crosshair's picture onto the frame as `src * (1 - dst) + dst *
/// (1 - src)`: where the crosshair is white, what's behind turns to its
/// opposite; where it's empty, what's behind stays. Its soft edges fade
/// between the two
const INVERT: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::OneMinusDst,
        dst_factor: BlendFactor::OneMinusSrc,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent::OVER,
};

/// The camera that draws only the crosshair
#[derive(Component)]
pub struct CrosshairCamera;

/// The crosshair's box, filling its camera's view
#[derive(Component)]
pub struct Crosshair;

/// A piece of the crosshair: the dot (`out` zero), or a line pointing out
/// from the centre along `out` (UI space: +y is down). `size` in UI pixels
#[derive(Component)]
pub struct CrosshairPart {
    out: Vec2,
    size: Vec2,
}

/// How far the lines have spread beyond their resting gap (UI pixels)
#[derive(Resource, Default)]
pub struct Spread(f32);

pub fn spawn_crosshair(mut cmds: Commands) {
    let camera = cmds
        .spawn((
            CrosshairCamera,
            Camera2d,
            Camera {
                // after the world (0) and the rest of the UI (1)
                order: 2,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                output_mode: CameraOutputMode::Write {
                    blend_state: Some(INVERT),
                    clear_color: ClearColorConfig::None,
                },
                ..default()
            },
            Msaa::Off,
        ))
        .id();

    let dot = CrosshairPart {
        out: Vec2::ZERO,
        size: Vec2::splat(CROSSHAIR_DOT),
    };
    let lines = [Vec2::NEG_Y, Vec2::Y, Vec2::NEG_X, Vec2::X].map(|out| CrosshairPart {
        out,
        size: if out.x == 0.0 {
            Vec2::new(CROSSHAIR_LINE_THICKNESS, CROSSHAIR_LINE_LENGTH)
        } else {
            Vec2::new(CROSSHAIR_LINE_LENGTH, CROSSHAIR_LINE_THICKNESS)
        },
    });
    cmds.spawn((
        Crosshair,
        UiTargetCamera(camera),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Visibility::Hidden,
    ))
    .with_children(|crosshair| {
        for part in std::iter::once(dot).chain(lines) {
            crosshair.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(part.size.x),
                    height: Val::Px(part.size.y),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                part,
            ));
        }
    });
}

/// Hidden while the game is paused or the inventory is open (the mouse is
/// free then, not aiming)
pub fn show_crosshair(info: Res<PlayerInfo>, mut crosshair: Single<&mut Visibility, With<Crosshair>>) {
    let shown = if info.paused || info.menu_open {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    crosshair.set_if_neq(shown);
}

/// Eases the spread toward our player's speed. The body's velocity, not how
/// the camera moves: a step lifts the body without giving it any speed, so
/// stairs don't throw the lines out
pub fn spread_crosshair(
    time: Res<Time>,
    player: Option<Single<&LinearVelocity, With<Controlled>>>,
    mut spread: ResMut<Spread>,
) {
    let target = player.map_or(0.0, |velocity| velocity.0.length() * CROSSHAIR_SPREAD_PER_SPEED);
    spread.0 += (target - spread.0) * (1.0 - (-CROSSHAIR_SETTLE * time.delta_secs()).exp());
}

/// Sizes the crosshair camera's box to fit the lines at the current spread,
/// centred in the window (never past its edges: lines further out are off
/// screen), then places the dot and lines in it on whole pixels, so they stay
/// sharp
pub fn place_crosshair(
    spread: Res<Spread>,
    window: Single<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut camera: Single<&mut Camera, With<CrosshairCamera>>,
    mut parts: Query<(&CrosshairPart, &mut Node)>,
) {
    let gap = (CROSSHAIR_GAP + spread.0).round();
    let scale = window.scale_factor() * ui_scale.0;
    let window_size = UVec2::new(window.physical_width(), window.physical_height());

    // wide enough for the lines each side, plus a margin. Kept the same
    // evenness as the window, so its middle lands on the window's middle
    let side = ((2.0 * (gap + CROSSHAIR_LINE_LENGTH + CROSSHAIR_PADDING)) * scale).ceil() as u32;
    let fit = |window: u32| (side + (side + window) % 2).min(window);
    let size = UVec2::new(fit(window_size.x), fit(window_size.y));
    let viewport = Viewport {
        physical_position: (window_size - size) / 2,
        physical_size: size,
        ..default()
    };
    let changed = camera.viewport.as_ref().is_none_or(|current| {
        current.physical_position != viewport.physical_position
            || current.physical_size != viewport.physical_size
    });
    if changed {
        camera.viewport = Some(viewport);
    }

    // the box's middle, in the UI pixels its contents are laid out in
    let center = size.as_vec2() / (2.0 * scale);
    for (part, mut node) in &mut parts {
        let middle = center + part.out * (gap + part.size.max_element() / 2.0);
        let corner = middle - part.size / 2.0;
        let (left, top) = (Val::Px(corner.x), Val::Px(corner.y));
        if node.left != left || node.top != top {
            node.left = left;
            node.top = top;
        }
    }
}
