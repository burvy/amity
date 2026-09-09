use avian3d::prelude::*;
use bevy::prelude::*;

pub const BASEPLATE_LENGTH: f32 = 50.0;
pub const BASEPLATE_THICKNESS: f32 = 5.0;

#[derive(Bundle)]
pub struct Baseplate {
    body: RigidBody,
    collider: Collider,
    position: Position,
}

impl Default for Baseplate {
    fn default() -> Self {
        Self {
            body: RigidBody::Static,
            collider: Collider::cuboid(BASEPLATE_LENGTH, BASEPLATE_THICKNESS, BASEPLATE_LENGTH),
            position: Position(Vec3 {
                x: 0.0,
                y: -5.0,
                z: 0.0,
            }),
        }
    }
}
