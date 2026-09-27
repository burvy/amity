use avian3d::schedule::PhysicsSystems;
use bevy::prelude::*;
use lightyear::prelude::{
    AppChannelExt, AppMessageExt, ChannelMode, ChannelSettings, NetworkDirection,
    ReliableSettings,
};

pub mod chunk;
pub mod definition;
pub mod edit;

/// Everything the voxel world registers on both client and server
pub struct WorldProtocolPlugin;

impl Plugin for WorldProtocolPlugin {
    fn build(&self, app: &mut App) {
        // sends big chunks to the player
        // to send block edits back we should just
        // use a different channel, EditChannel since
        // it is cleaner.
        // lost packets are resent until they arrive
        app.add_channel::<chunk::WorldChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);

        app.register_message::<chunk::ChunkData>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<edit::BlockChanged>()
            .add_direction(NetworkDirection::ServerToClient);

        app.init_resource::<chunk::ChunkIndex>();
        app.add_observer(chunk::index_chunk);
        app.add_observer(chunk::unindex_chunk);
        app.add_systems(
            FixedPostUpdate,
            edit::sync_colliders.before(PhysicsSystems::First),
        );
    }
}
