use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Bundle)]
pub struct InteractableBox {
    body: RigidBody,
    collider: Collider,
    position: Position,
}

impl Default for InteractableBox {
    fn default() -> Self {
        Self {
            body: RigidBody::Static,
            collider: Collider::cuboid(1.0, 1.0, 1.0),
            position: Position(Vec3 {
                x: 0.0,
                y: 2.0,
                z: 0.0,
            }),
        }
    }
}
