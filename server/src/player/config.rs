use amity_protocol::player::config::PLAYER_HALF_HEIGHT;
use bevy::prelude::*;

/// Where players appear when they join. Also where rolling pregeneration starts,
/// so the ground here exists before anyone arrives
/// TODO: replace this spawnpoint with actual set spawn later on
pub const SPAWN_POINT: Vec3 = Vec3::new(0.0, PLAYER_HALF_HEIGHT, 0.0);

/// ticks before we stop replaying the client's last action after they stop
/// sending inputs. 128 ticks for 64Hz is 2 seconds.
pub const STALE_INPUT_TICKS: i32 = 128;
