use avian3d::prelude::*;
use bevy::prelude::*;

pub const BOX_SIZE: f32 = 1.0;

#[derive(Bundle)]
pub struct InteractableBox {
    body: RigidBody,
    collider: Collider,
}

impl Default for InteractableBox {
    fn default() -> Self {
        Self {
            body: RigidBody::Static,
            collider: Collider::cuboid(BOX_SIZE, BOX_SIZE, BOX_SIZE),
        }
    }
}
