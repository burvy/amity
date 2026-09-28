use amity_protocol::player::{
    config::{EYE_HEIGHT, PLAYER_HEIGHT, PLAYER_WIDTH},
    definition::{PlayerBody, PlayerInputs, PlayerMarker},
    movement::Stepped,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::{
    frame_interpolation::FrameInterpolate,
    prelude::{
        input::native::{ActionState, InputMarker},
        *,
    },
};

use super::{
    definition::{MainPlayerCamera, PlayerInfo},
    glide::StepGlide,
};

/// Insert physics onto player upon being newly predicted
pub fn add_physics(
    mut cmds: Commands,
    players: Query<Entity, (Added<Predicted>, With<PlayerMarker>)>,
) {
    for player in players.iter() {
        cmds.entity(player).insert((
            PlayerBody::default(), // add ONCE
            // players move 64 times a second, but frames come at any rate: show
            // them one tick behind, blended between the last two ticks, so they
            // glide instead of judder
            FrameInterpolate,
            // steps up and down, smoothed for the camera and model
            Stepped::default(),
            StepGlide::default(),
        ));
    }
}

/// Marks a player's visible model: a child of their body, so it can turn to
/// face where they look while the body's box stays square to the world
#[derive(Component)]
pub struct PlayerModel;

/// Insert player model onto player upon being newly predicted
pub fn draw_players(
    mut cmds: Commands,
    players: Query<Entity, (Added<Predicted>, With<PlayerMarker>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for player in players.iter() {
        cmds.entity(player).insert(Visibility::default()).with_child((
            PlayerModel,
            Mesh3d(meshes.add(Cuboid::new(PLAYER_WIDTH, PLAYER_HEIGHT, PLAYER_WIDTH))),
            MeshMaterial3d(materials.add(Color::WHITE)),
        ));
    }
}

/// Turns each player's model to the way they look (yaw only), and sets it
/// gliding through steps. Runs before transforms are handed to rendering
pub fn pose_models(
    mut models: Query<(&ChildOf, &mut Transform), With<PlayerModel>>,
    players: Query<(&ActionState<PlayerInputs>, &StepGlide)>,
) {
    for (parent, mut transform) in &mut models {
        if let Ok((action, glide)) = players.get(parent.parent()) {
            transform.rotation = Quat::from_rotation_y(action.0.look.x);
            transform.translation.y = glide.offset;
        }
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

/// Puts the camera at our player's eye. Runs after frame interpolation and
/// step gliding, so it reads the smoothed position rather than the last tick's
pub fn camera_follow_player(
    mut camera: Single<&mut Transform, With<MainPlayerCamera>>,
    player: Single<(&Position, &StepGlide), With<Controlled>>,
    look_res: Res<PlayerInfo>,
) {
    let (position, glide) = *player;
    camera.translation = position.0 + Vec3::Y * (EYE_HEIGHT + glide.offset);
    camera.rotation = Quat::from_euler(EulerRot::YXZ, look_res.look.x, look_res.look.y, 0.0);
}
