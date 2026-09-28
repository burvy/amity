//! The voxel world: blocks, chunks, their colliders, and the messages that sync
//! them. Everything the client and server both need

use avian3d::schedule::PhysicsSystems;
use bevy::prelude::*;
use lightyear::prelude::{
    AppChannelExt, AppMessageExt, ChannelMode, ChannelSettings, NetworkDirection,
    ReliableSettings,
};

pub mod block;
pub mod blocks;
pub mod chunk;
pub mod config;
pub mod edit;
pub mod messages;
pub mod structure;

/// Everything the voxel world registers on both client and server
pub struct WorldProtocolPlugin;

impl Plugin for WorldProtocolPlugin {
    fn build(&self, app: &mut App) {
        // lost packets are resent until they arrive
        app.add_channel::<messages::WorldChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<messages::ChunkData>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<messages::UnloadChunk>()
            .add_direction(NetworkDirection::ServerToClient);
        app.add_channel::<messages::BlockChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<messages::BlockChanged>()
            .add_direction(NetworkDirection::ServerToClient);

        app.add_channel::<messages::EditChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<messages::EditBlock>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<messages::SetViewDistance>()
            .add_direction(NetworkDirection::ClientToServer);

        app.add_message::<blocks::BlockSet>();
        app.init_resource::<chunk::ChunkIndex>();
        app.add_observer(chunk::pos::index_chunk);
        app.add_observer(chunk::pos::unindex_chunk);
        app.add_systems(
            FixedPostUpdate,
            (chunk::collider::init_colliders, chunk::collider::update_colliders)
                .chain()
                .before(PhysicsSystems::First),
        );
    }
}
