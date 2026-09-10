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
        DirectionalLight::default(),
        Transform::from_translation(Vec3::new(5.0, 5.0, 10.0)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Draw interactable entities
/// TODO: for the mesh and material, eventually use an enum that determines what to draw
/// instead of hardcoding it like it is right now
pub fn draw_interactables(
    mut cmds: Commands,
    interactables: Query<Entity, Added<stuff::definition::Interactable>>,
    mut mesh: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for interactable in &interactables {
        cmds.entity(interactable).insert((
            // TODO: use enum based on component to determine what to draw
            stuff::ibox::definition::InteractableBox::default(),
            Mesh3d(mesh.add(Cuboid::new(
                stuff::ibox::definition::BOX_SIZE,
                stuff::ibox::definition::BOX_SIZE,
                stuff::ibox::definition::BOX_SIZE,
            ))),
            MeshMaterial3d::<StandardMaterial>(mats.add(Color::srgb_u8(0, 128, 255))),
        ));
    }
}

/// Draw a debug grid (local only)
pub fn draw_grid(mut gizmos: Gizmos) {
    gizmos.grid(
        Isometry3d::new(Vec3::new(0.0, -2.49, 0.0), Quat::from_rotation_x(FRAC_PI_2)),
        UVec2::splat(world::definition::BASEPLATE_LENGTH as u32),
        Vec2::splat(1.0),
        Color::srgb_u8(255, 0, 0),
    );
}
