use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use shooter_protocol::{stuff, world};

pub fn build_baseplate(
    mut cmds: Commands,
    mut mesh: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    // world building
    cmds.spawn((
        world::definition::Baseplate::default(),
        Mesh3d(mesh.add(Cuboid::new(
            world::definition::BASEPLATE_LENGTH,
            world::definition::BASEPLATE_THICKNESS,
            world::definition::BASEPLATE_LENGTH,
        ))),
        MeshMaterial3d::<StandardMaterial>(mats.add(Color::srgb_u8(255, 128, 64))),
    ));

    cmds.spawn((
        stuff::ibox::definition::InteractableBox::default(),
        Mesh3d(mesh.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d::<StandardMaterial>(mats.add(Color::srgb_u8(0, 128, 255))),
    ));

    cmds.spawn((
        DirectionalLight::default(),
        Transform::from_translation(Vec3::new(5.0, 5.0, 10.0)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Draw a debug grid
pub fn draw_grid(mut gizmos: Gizmos) {
    gizmos.grid(
        Isometry3d::new(Vec3::new(0.0, -2.49, 0.0), Quat::from_rotation_x(FRAC_PI_2)),
        UVec2::splat(world::definition::BASEPLATE_LENGTH as u32),
        Vec2::splat(1.0),
        Color::srgb_u8(255, 0, 0),
    );
}
