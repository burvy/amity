//! Capturing structures (dev builds only): build something in game, mark the
//! box around it, and save it as a structure file for world generation.
//!
//! - **F6**: first corner, at the block in the crosshair
//! - **F7**: the opposite corner
//! - **F8**: save everything between them (corners included) to
//!   `assets/structures/captured_<time>.ron`. Rename it (the name is what the
//!   generation settings use), and restart the server to pack it in
//!
//! How to box a tree: put the box's bottom layer at the ground under it. That
//! layer is never placed, it only marks where the ground goes, so the trunk
//! should start on the layer above it. Whatever stands on that ground (the
//! trunk) is what's planted on the spot, wherever it is in the box. Air in the
//! box is left alone when placed, so box loosely. See protocol
//! `world/structure.rs` for the file format.

use amity_protocol::{
    player::aim,
    world::{blocks::Blocks, config::REACH, edit::EditTarget, structure::Structure},
};
use avian3d::prelude::{Position, SpatialQuery};
use bevy::prelude::*;
use lightyear::prelude::Controlled;

use crate::player::definition::PlayerInfo;

/// Where the captured files go (relative to where the game runs: the repo)
const FOLDER: &str = "assets/structures";

/// The two corners picked so far
#[derive(Resource, Default)]
pub struct Capture {
    first: Option<IVec3>,
    second: Option<IVec3>,
}

/// F6 and F7 pick the corners, F8 saves the box
pub fn capture(
    keys: Res<ButtonInput<KeyCode>>,
    mut capture: ResMut<Capture>,
    player: Option<Single<(Entity, &Position), With<Controlled>>>,
    info: Res<PlayerInfo>,
    spatial: SpatialQuery,
    blocks: Blocks,
) {
    let corner = keys.just_pressed(KeyCode::F6) || keys.just_pressed(KeyCode::F7);
    if corner {
        let Some((entity, position)) = player.map(|player| *player) else {
            return;
        };
        let Some(hit) = aim::aim(&spatial, entity, position.0, info.look, REACH)
            .filter(|aim| blocks.is_chunk(aim.entity))
            .map(|aim| EditTarget::from_aim(&aim).hit)
        else {
            info!("capture: no block in the crosshair");
            return;
        };
        if keys.just_pressed(KeyCode::F6) {
            capture.first = Some(hit);
            info!("capture: first corner {hit}");
        } else {
            capture.second = Some(hit);
            info!("capture: second corner {hit}");
        }
    }

    if keys.just_pressed(KeyCode::F8) {
        let (Some(a), Some(b)) = (capture.first, capture.second) else {
            info!("capture: pick both corners first (F6, F7)");
            return;
        };
        match save(a.min(b), a.max(b), &blocks) {
            Ok(path) => info!("capture: saved {path}"),
            Err(e) => warn!("capture: {e}"),
        }
    }
}

/// Writes the blocks from `low` to `high` (both included) to a new file, and
/// returns its path
fn save(low: IVec3, high: IVec3, blocks: &Blocks) -> Result<String, String> {
    let size = high - low + IVec3::ONE;
    if size.y < 2 {
        return Err("the box needs at least 2 layers: the ground and something on it".into());
    }
    let mut structure = Structure::empty(size);
    for y in 0..size.y {
        for z in 0..size.z {
            for x in 0..size.x {
                let p = IVec3::new(x, y, z);
                let block = blocks
                    .get(low + p)
                    .ok_or_else(|| format!("the block at {} isn't loaded", low + p))?;
                structure.set(p, block);
            }
        }
    }
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_secs());
    let path = format!("{FOLDER}/captured_{seconds}.ron");
    std::fs::create_dir_all(FOLDER).map_err(|e| format!("can't make {FOLDER}: {e}"))?;
    std::fs::write(&path, structure.to_ron()).map_err(|e| format!("can't write {path}: {e}"))?;
    Ok(path)
}
