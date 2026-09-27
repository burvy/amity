use bevy::prelude::*;

pub fn spawn_light(mut cmds: Commands) {
    cmds.spawn((
        DirectionalLight::default(),
        Transform::from_translation(Vec3::new(5.0, 5.0, 10.0)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
