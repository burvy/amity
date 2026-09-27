use amity_protocol::player::{
    config::{EYE_HEIGHT, PLAYER_LENGTH, PLAYER_RADIUS},
    definition::{PlayerBody, PlayerInputs, PlayerMarker},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::{input::native::InputMarker, *};

use super::definition::{MainPlayerCamera, PlayerInfo};

/// Insert physics onto player upon being newly predicted
pub fn add_physics(
    mut cmds: Commands,
    players: Query<Entity, (Added<Predicted>, With<PlayerMarker>)>,
) {
    for player in players.iter() {
        cmds.entity(player).insert(
            PlayerBody::default(), // add ONCE
        );
    }
}

/// Insert player model onto player upon being newly predicted
pub fn draw_players(
    mut cmds: Commands,
    players: Query<Entity, (Added<Predicted>, With<PlayerMarker>)>,
) {
    for player in players.iter() {
        cmds.entity(player).queue_apply_scene(bsn! {
            Mesh3d(asset_value(Capsule3d::new(PLAYER_RADIUS, PLAYER_LENGTH)))
            MeshMaterial3d::<StandardMaterial>(asset_value(Color::WHITE))
        });
    }
}

/// Logic to run when any player joining is detected
pub fn detect_replicate_player(player: On<Add, PlayerMarker>) {
    info!("Player {:?} was replicated to me!", player.entity);
}

/// Logic to run when our designated controlled player joins
pub fn detect_replicate_our_player(our_player: On<Add, Controlled>, mut cmds: Commands) {
    cmds.entity(our_player.entity)
        .insert(InputMarker::<PlayerInputs>::default());
}

pub fn camera_follow_player(
    mut camera: Single<&mut Transform, With<MainPlayerCamera>>,
    player: Single<&Position, With<Controlled>>,
    look_res: Res<PlayerInfo>,
) {
    camera.translation = player.0 + Vec3::Y * EYE_HEIGHT;
    camera.rotation = Quat::from_euler(EulerRot::YXZ, look_res.look.x, look_res.look.y, 0.0);
}
