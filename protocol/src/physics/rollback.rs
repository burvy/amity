//! When a client's prediction is far enough from the server to roll back

use avian3d::{dynamics::rigid_body::LinearVelocity, physics_transform::Position};

use super::config::{ROLLBACK_POSITION, ROLLBACK_VELOCITY};

pub fn position(this: &Position, that: &Position) -> bool {
    (this.0 - that.0).length() >= ROLLBACK_POSITION
}

pub fn velocity(this: &LinearVelocity, that: &LinearVelocity) -> bool {
    (this.0 - that.0).length() >= ROLLBACK_VELOCITY
}
