use amity_protocol::inventory::{definition::GameMode, item::Item};
use bevy::prelude::*;

/// Where players appear when they join (x, z metres), standing on the ground.
/// If that's ocean, the nearest land is used instead. Also where rolling
/// pregeneration starts, so that ground exists before anyone arrives
/// TODO: replace this spawnpoint with actual set spawn later on
pub const SPAWN_XZ: Vec2 = Vec2::ZERO;

/// How players play when they join
/// TODO: per-player and per-world, once there are accounts and save files
pub const DEFAULT_GAME_MODE: GameMode = GameMode::Creative;

/// What a new player's hotbar holds (a full stack of each), slot by slot
pub const STARTING_HOTBAR: [Item; 5] = [
    Item::STONE,
    Item::DIRT,
    Item::PODZOL,
    Item::SAND,
    Item::DRY_GRASS,
];

/// ticks before we stop replaying the client's last action after they stop
/// sending inputs. 128 ticks for 64Hz is 2 seconds.
pub const STALE_INPUT_TICKS: i32 = 128;
