use std::time::Duration;

use crate::{generation, logic};
use avian3d::physics_transform::Position;
use bevy::prelude::*;
use lightyear::input::input_buffer::InputBuffer;
use lightyear::input::server::{authorize_controlled_targets, InputValidationAppExt};
use lightyear::prelude::input::native::{ActionState, NativeStateSequence};
use lightyear::prelude::server::ServerPlugins;
use lightyear::prelude::*;
use lightyear::{
    connection::{client::Connected, client_of::ClientOf, server::Start},
    netcode::{server_plugin::NetcodeConfig, NetcodeServer},
    webtransport::server::WebTransportServerIo,
};
use amity_protocol::world::chunk::{self, Chunk, ChunkData, ChunkPos};
use amity_protocol::{protocol, shared, stuff};
// use powershell:
// Install-Module -Name Posh-ACME -Scope CurrentUser
// Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
// Import-Module Posh-ACME
// Set-PAServer LE_PROD
// New-PACertificate '192.0.2.1(replace with your public ip)' `
//   -Plugin WebRoot `
//   -PluginArgs @{ WRPath = 'C:\acme-challenge' } `
//   -Profile shortlived `
//   -AcceptTOS `
//   -Contact 'name@email.com(replace with your email)'
#[cfg(not(feature = "dev-local"))]
const KEY_PATH: &str =
    r"C:\Users\Burvy\AppData\Local\Posh-ACME\LE_PROD\3716529536\174.175.161.63\cert.key";
#[cfg(not(feature = "dev-local"))]
const CERT_PATH: &str =
    r"C:\Users\Burvy\AppData\Local\Posh-ACME\LE_PROD\3716529536\174.175.161.63\fullchain.cer";

// ticks before we stop replaying the client's last action after they stop
// sending inputs. 128 ticks for 64Hz is 2 seconds.
const STALE_INPUT_TICKS: i32 = 128;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(protocol::TIMESTEP),
        });
        app.add_observer(on_connect);

        app.add_systems(
            FixedUpdate,
            clear_stale_input.before(logic::server_player_motion),
        );
        app.add_systems(
            FixedUpdate,
            (
                logic::server_player_motion,
                shared::step_up::<With<protocol::PlayerMarker>>,
                logic::server_interact,
                logic::keep_players_replicating,
            )
                .chain(),
        );
        app.add_systems(Update, (heartbeat, send_world, logic::receive_edits));

        // this is not default, but prevents clients from controlling
        // entities they shouldn't be on the server
        app.add_input_validator(
            authorize_controlled_targets::<NativeStateSequence<protocol::PlayerInputs>>,
        );
    }
}

/// Initializes server connection, world, etc
fn startup(mut cmds: Commands) -> Result {
    // real certificate instead of using digest
    #[cfg(not(feature = "dev-local"))]
    let identity = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(Identity::load_pemfiles(CERT_PATH, KEY_PATH))?;

    // offline dev: throwaway certificate generated at startup, no files and no
    // network. the client's dev-local skips validation, so contents don't matter.
    #[cfg(feature = "dev-local")]
    let identity = Identity::self_signed(["localhost", "127.0.0.1"])?;

    let server = cmds
        .spawn((
            NetcodeServer::new(NetcodeConfig {
                client_timeout_secs: 300, // under manual authentication, is inert
                ..default()
            }),
            LocalAddr(protocol::SERVER_BIND_ADDR),
            WebTransportServerIo {
                certificate: identity,
            },
        ))
        .id();

    // world building
    for x in -1..=0 {
        for y in -1..=1 {
            // adjust the y lower bound for the build limit in chunks
            for z in -1..=0 {
                let pos = ChunkPos(IVec3::new(x, y, z));
                cmds.spawn(chunk::chunk_body(pos, generation::generate(pos)));
            }
        }
    }
    cmds.spawn((
        stuff::ibox::definition::InteractableBox::default(),
        stuff::definition::Interactable {
            delay_secs: 1.0,
            last_interacted: Tick(0),
        },
        Position(Vec3::new(0.0, 1.0, 0.0)),
        Replicate::to_clients(NetworkTarget::All),
    ));

    cmds.trigger(Start { entity: server });
    Ok(())
}

/// Attaches the replication outbox to a newly connected client and spawns the player
fn on_connect(
    trigger: On<Add, Connected>,      // triggers when someone connected
    query: Query<(), With<ClientOf>>, // query `RemoteId`s that connected to us
    mut cmds: Commands,
) {
    if !query.contains(trigger.entity) {
        // client connected that isn't a client of us
        return;
    };

    // `ControlledBy` searches for `With<ReplicationSender>`, so this is required
    cmds.entity(trigger.entity).insert(ReplicationSender);
    cmds.spawn((
        // for player querying/identification
        protocol::PlayerMarker,
        // TODO: replace this spawnpoint with actual set spawn later on
        Position(Vec3::new(0.0, shared::PLAYER_HALF_HEIGHT, 0.0)),
        // agreed-upon player body shape between client and server
        shared::PlayerBody::default(),
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

/// Sends every chunk to newly connected clients
fn send_world(
    mut clients: Query<&mut MessageSender<ChunkData>, (Added<Connected>, With<ClientOf>)>,
    chunks: Query<(&ChunkPos, &Chunk)>,
) {
    for mut sender in &mut clients {
        for (pos, chunk) in &chunks {
            sender.send::<chunk::WorldChannel>(ChunkData {
                pos: *pos,
                chunk: chunk.clone(),
            });
        }
        info!("sent {} chunks", chunks.iter().count());
    }
}

/// Logging function to show connected things every minute
fn heartbeat(
    clients: Query<(), (With<ClientOf>, With<Connected>)>,
    senders: Query<(), With<ReplicationSender>>,
    players: Query<(), With<protocol::PlayerMarker>>,
    time: Res<Time>,
    mut next: Local<f32>,
) {
    if time.elapsed_secs() < *next {
        return;
    }

    *next = time.elapsed_secs() + 60.0;
    info!(
        "clients: {}, senders: {}, players: {}",
        clients.iter().count(),
        senders.iter().count(),
        players.iter().count(),
    )
}

/// Per `lightyear_inputs/src/server.rs:706`, frozen clients stop sending input but
/// lightyear replays their last action forever. We have a long timeout, and should
/// zero the input once they stop sending input.
/// This function sets input to default if the player has been quiet for STALE_INPUT_TICKS
/// and they have some queued input from lightyear due to freezing.
fn clear_stale_input(
    timeline: Res<LocalTimeline>,
    mut players: Query<
        (
            &mut ActionState<protocol::PlayerInputs>,
            &InputBuffer<ActionState<protocol::PlayerInputs>, protocol::PlayerInputs>,
        ),
        With<protocol::PlayerMarker>,
    >,
) {
    let tick = timeline.tick();
    for (mut action, buffer) in &mut players {
        // no input or last input was longer than STALE_INPUT_TICKS
        let quiet = buffer
            .last_remote_tick
            .is_none_or(|last| tick - last > STALE_INPUT_TICKS);

        // catches when players just have lightyear sending their last action while quiet
        if quiet && action.0 != protocol::PlayerInputs::default() {
            action.0 = protocol::PlayerInputs::default();
        }
    }
}
