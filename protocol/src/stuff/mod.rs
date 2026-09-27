//! Things in the world players can use, like the interactable box

use bevy::prelude::*;
use lightyear::prelude::AppComponentExt;

pub mod definition;
pub mod ibox;

pub struct StuffProtocolPlugin;

impl Plugin for StuffProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<definition::Interactable>().replicate();
    }
}
