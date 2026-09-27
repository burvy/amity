//! Items, the hotbar, game modes and flying

use bevy::prelude::*;
use lightyear::prelude::{
    AppChannelExt, AppComponentExt, AppMessageExt, ChannelMode, ChannelSettings,
    NetworkDirection, ReliableSettings,
};
use lightyear::prediction::registry::PredictionBuilderExt;

pub mod config;
pub mod definition;
pub mod item;

pub struct InventoryProtocolPlugin;

impl Plugin for InventoryProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<definition::Hotbar>().replicate();
        app.component::<definition::GameMode>().replicate();
        // movement depends on it, so clients predict it like position
        app.component::<definition::Flying>().replicate().predict();

        app.add_channel::<definition::ActionChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<definition::SetSlot>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<definition::ToggleFlight>()
            .add_direction(NetworkDirection::ClientToServer);
    }
}
