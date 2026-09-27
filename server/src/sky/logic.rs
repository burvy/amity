use amity_protocol::{
    net::config::TIMESTEP,
    sky::{
        config::{DAY_LENGTH_SECS, START_DAY_OF_YEAR, START_TIME_OF_DAY},
        WorldClock,
    },
};
use bevy::prelude::*;
use lightyear::prelude::*;

/// Real seconds between clock updates sent to clients. Clients advance their
/// own copy in between, so this only corrects drift
const PUBLISH_EVERY_SECS: f64 = 1.0;

/// The exact time, advanced every tick. `WorldClock` (replicated) is only set
/// from it every `PUBLISH_EVERY_SECS`, since every change is sent to every client
#[derive(Resource)]
pub struct Clock {
    days: f64,
    /// ticks since `WorldClock` was last set
    since_publish: u32,
}

impl Default for Clock {
    fn default() -> Self {
        // TODO: saved with the world, once there are save files
        Self {
            days: START_DAY_OF_YEAR + START_TIME_OF_DAY,
            since_publish: 0,
        }
    }
}

/// The one entity carrying `WorldClock`, replicated to everyone
pub fn spawn_clock(mut cmds: Commands, clock: Res<Clock>) {
    cmds.spawn((
        WorldClock(clock.days),
        Replicate::to_clients(NetworkTarget::All),
    ));
}

pub fn advance_clock(mut clock: ResMut<Clock>, mut world_clock: Single<&mut WorldClock>) {
    clock.days += TIMESTEP / DAY_LENGTH_SECS;
    clock.since_publish += 1;
    if clock.since_publish as f64 * TIMESTEP >= PUBLISH_EVERY_SECS {
        clock.since_publish = 0;
        world_clock.0 = clock.days;
    }
}
