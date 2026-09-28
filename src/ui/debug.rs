//! The F3 debug screen: frame rate, where you are and what you're looking at,
//! chunks, connection, and the time and sky

use std::f64::consts::TAU;

use amity_protocol::{
    inventory::definition::{Flying, GameMode},
    player::aim,
    sky::astronomy::{moon_illumination, moon_phase, sun_direction},
    world::{
        block::block_at,
        chunk::{Chunk, ChunkIndex, ChunkPos},
        config::REACH,
        edit::EditTarget,
    },
};
use avian3d::prelude::{Position, SpatialQuery};
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use lightyear::prelude::{client::Client, Controlled, Link};

use super::{config::*, font::UiFont};
use crate::{
    player::{config::KEY_DEBUG, definition::PlayerInfo},
    sky::logic::SkyTime,
    world::{chunks::IncomingChunks, meshing::tasks::MeshTask},
};

/// The debug screen's text
#[derive(Component)]
pub struct DebugText;

/// Seconds between refreshes: rebuilding the text every frame costs more than
/// it's worth, and numbers changing 60 times a second can't be read anyway
const REFRESH_SECS: f32 = 0.25;

pub fn spawn_debug(mut cmds: Commands, font: Res<UiFont>) {
    cmds.spawn((
        DebugText,
        Text::new(""),
        TextFont {
            font: font.0.clone().into(),
            font_size: FontSize::Px(DEBUG_FONT_SIZE),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(DEBUG_MARGIN),
            top: Val::Px(DEBUG_MARGIN),
            padding: UiRect::all(Val::Px(DEBUG_PADDING)),
            ..default()
        },
        BackgroundColor(DEBUG_BACKGROUND),
        Visibility::Hidden,
    ));
}

/// F3 shows and hides it
pub fn toggle_debug(keys: Res<ButtonInput<KeyCode>>, mut text: Single<&mut Visibility, With<DebugText>>) {
    if keys.just_pressed(KEY_DEBUG) {
        **text = match **text {
            Visibility::Hidden => Visibility::Inherited,
            _ => Visibility::Hidden,
        };
    }
}

pub fn update_debug(
    mut text: Single<(&mut Text, &Visibility), With<DebugText>>,
    time: Res<Time>,
    mut next: Local<f32>,
    diagnostics: Res<DiagnosticsStore>,
    info: Res<PlayerInfo>,
    player: Option<Single<(Entity, &Position, Option<&GameMode>, Option<&Flying>), With<Controlled>>>,
    spatial: SpatialQuery,
    index: Res<ChunkIndex>,
    chunks: Query<&Chunk>,
    incoming: Res<IncomingChunks>,
    meshing: Query<(), With<MeshTask>>,
    link: Option<Single<&Link, With<Client>>>,
    sky: Res<SkyTime>,
) {
    let (text, visibility) = &mut *text;
    if **visibility == Visibility::Hidden || time.elapsed_secs() < *next {
        return;
    }
    *next = time.elapsed_secs() + REFRESH_SECS;

    let mut lines = Vec::new();
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .unwrap_or(0.0);
    lines.push(format!("Amity  {fps:.0} fps ({:.1} ms)", 1000.0 / fps.max(1.0)));

    if let Some(player) = player {
        let (entity, position, mode, flying) = player.into_inner();
        let p = position.0;
        let block = block_at(p);
        let chunk = ChunkPos::of_block(block).0;
        lines.push(format!("XYZ {:.1} / {:.1} / {:.1} m", p.x, p.y, p.z));
        lines.push(format!("Block {} {} {}   Chunk {} {} {}", block.x, block.y, block.z, chunk.0.x, chunk.0.y, chunk.0.z));
        lines.push(format!(
            "Facing {}   yaw {:.0}°  pitch {:.0}°",
            facing(info.look.x),
            info.look.x.to_degrees().rem_euclid(360.0),
            info.look.y.to_degrees()
        ));

        // the block in the crosshair, the same way breaking finds it
        let target = aim::aim(&spatial, entity, p, info.look, REACH)
            .filter(|aim| chunks.contains(aim.entity))
            .map(|aim| EditTarget::from_aim(&aim).hit);
        lines.push(match target {
            Some(target) => {
                let (chunk, local) = ChunkPos::of_block(target);
                let block = index
                    .get(chunk)
                    .and_then(|entity| chunks.get(entity).ok())
                    .map(|chunk| chunk.get(local));
                format!("Looking at {block:?} at {} {} {}", target.x, target.y, target.z)
            }
            None => "Looking at nothing".into(),
        });

        let mode = match mode {
            Some(GameMode::Creative) => "creative",
            Some(GameMode::Survival) => "survival",
            None => "?",
        };
        let flying = if flying.is_some_and(|flying| flying.0) { ", flying" } else { "" };
        lines.push(format!("Mode {mode}{flying}"));
    }

    lines.push(format!(
        "Chunks {} loaded, {} arriving, {} meshing",
        index.len(),
        incoming.len(),
        meshing.iter().count()
    ));
    if let Some(link) = link {
        lines.push(format!("Ping {:.0} ms", link.stats.rtt.as_secs_f64() * 1000.0));
    }

    if let Some(days) = sky.days() {
        let day_of_year = days.floor() as i64 % 365;
        let minutes = (days.fract() * 24.0 * 60.0) as u32;
        let sun = sun_direction(days).y.asin().to_degrees();
        let phase = moon_phase(days);
        let waxing = if phase < 0.5 { "waxing" } else { "waning" };
        lines.push(format!(
            "Day {day_of_year} ({}), {:02}:{:02}   sun {sun:.0}° up",
            season(day_of_year),
            minutes / 60,
            minutes % 60
        ));
        lines.push(format!(
            "Moon {:.0}% lit, {waxing}",
            moon_illumination(days) * 100.0
        ));
    }

    let joined = lines.join("\n");
    if text.0 != joined {
        text.0 = joined;
    }
}

/// The compass direction of a yaw (radians, 0 looking along -Z, which is north)
fn facing(yaw: f32) -> &'static str {
    const NAMES: [&str; 8] = [
        "north", "north-west", "west", "south-west", "south", "south-east", "east", "north-east",
    ];
    // yaw grows turning left (towards west)
    let eighth = ((yaw as f64).rem_euclid(TAU) / TAU * 8.0).round() as usize % 8;
    NAMES[eighth]
}

/// The northern hemisphere's season on a day of the year
fn season(day_of_year: i64) -> &'static str {
    match day_of_year {
        80..=171 => "spring",
        172..=265 => "summer",
        266..=354 => "autumn",
        _ => "winter",
    }
}
