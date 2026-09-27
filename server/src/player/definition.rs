use bevy::prelude::*;

/// Where players appear: on top of the ground, on land, as near `SPAWN_XZ` as
/// possible. Found once at startup by the generation plugin, since searching for
/// land samples a lot of noise
#[derive(Resource, Clone, Copy, Debug)]
pub struct SpawnPoint(pub Vec3);
