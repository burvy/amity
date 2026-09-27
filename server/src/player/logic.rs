use amity_protocol::player::{
    config::PLAYER_HALF_HEIGHT,
    definition::{PlayerBody, PlayerInputs, PlayerMarker},
};
use avian3d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use lightyear::connection::{client::Connected, client_of::ClientOf};
use lightyear::input::input_buffer::InputBuffer;
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::*;

use super::config::{SPAWN_XZ, STALE_INPUT_TICKS};
use crate::world::generation::terrain::Terrain;

/// Spawns a player for each newly connected client
pub fn spawn_player(
    trigger: On<Add, Connected>,      // triggers when someone connected
    query: Query<(), With<ClientOf>>, // query `RemoteId`s that connected to us
    terrain: Res<Terrain>,
    mut cmds: Commands,
) {
    if !query.contains(trigger.entity) {
        // client connected that isn't a client of us
        return;
    };

    // `ControlledBy` searches for `With<ReplicationSender>`, so this is required.
    // Inserted here, before the player, so it's there when `ControlledBy` looks
    cmds.entity(trigger.entity).insert(ReplicationSender);
    cmds.spawn((
        // for player querying/identification
        PlayerMarker,
        // standing on the ground: the body's centre is half its height up
        Position(terrain.surface_point(SPAWN_XZ) + Vec3::Y * PLAYER_HALF_HEIGHT),
        // agreed-upon player body shape between client and server
        PlayerBody::default(),
        // replicate for everyone
        Replicate::to_clients(NetworkTarget::All),
        // make all players predict
        PredictionTarget::to_clients(NetworkTarget::All),
        // Server puts `ControlledBy` on player entity, which is *replicated*
        // to client, who gets a `Controlled` marker. The client then writes
        // `ActionState<PlayerInputs>` into the `Controlled` entity, ships it
        // to the server while tagged with the ID, which is mapped back to the
        // server entity's id where its own `ActionState` is written.
        // `movement` reads and acts on the written `ActionState`
        ControlledBy {
            owner: trigger.entity,        // connection that owns this entity
            lifetime: Default::default(), // despawn upon disconnect (SessionBased)
        },
    ));
}

/// Per `lightyear_inputs/src/server.rs:706`, frozen clients stop sending input but
/// lightyear replays their last action forever. We have a long timeout, and should
/// zero the input once they stop sending input.
/// This function sets input to default if the player has been quiet for STALE_INPUT_TICKS
/// and they have some queued input from lightyear due to freezing.
pub fn clear_stale_input(
    timeline: Res<LocalTimeline>,
    mut players: Query<
        (
            &mut ActionState<PlayerInputs>,
            &InputBuffer<ActionState<PlayerInputs>, PlayerInputs>,
        ),
        With<PlayerMarker>,
    >,
) {
    let tick = timeline.tick();
    for (mut action, buffer) in &mut players {
        // no input or last input was longer than STALE_INPUT_TICKS
        let quiet = buffer
            .last_remote_tick
            .is_none_or(|last| tick - last > STALE_INPUT_TICKS);

        // catches when players just have lightyear sending their last action while quiet
        if quiet && action.0 != PlayerInputs::default() {
            action.0 = PlayerInputs::default();
        }
    }
}

/// Marks every player's physics state changed each tick, so lightyear replicates it
/// even when it isn't moving. Clients only check their prediction when new server
/// state arrives; without this, a player that diverged while standing still would
/// never be corrected.
pub fn keep_players_replicating(
    mut players: Query<(&mut Position, &mut LinearVelocity), With<PlayerMarker>>,
) {
    for (mut position, mut velocity) in &mut players {
        position.set_changed();
        velocity.set_changed();
    }
}
