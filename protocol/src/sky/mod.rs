//! Time of day, seasons, and where the sun and moon are
//!
//! - `config.rs`: day length, latitude, the world's starting date
//! - `astronomy.rs`: the sun's and moon's directions from the date and time
//! - `WorldClock`: the time, owned by the server and replicated

use bevy::prelude::*;
use lightyear::prelude::AppComponentExt;
use serde::{Deserialize, Serialize};

pub mod astronomy;
pub mod config;

/// The world's time: days since 1 January of year 0, plus the fraction of the
/// current day (0.5 is noon). Lives on one entity, owned by the server, which
/// updates it about once a second; clients run their own copy smoothly between
/// updates (see client `sky`)
#[derive(Component, Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct WorldClock(pub f64);

pub struct SkyProtocolPlugin;

impl Plugin for SkyProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<WorldClock>().replicate();
    }
}
