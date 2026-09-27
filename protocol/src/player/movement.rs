//! Walking, sprinting, jumping, air strafing and flying

use avian3d::prelude::*;
use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::input::native::ActionState;

use super::{
    config::{AIR_ACCEL, JUMP_VEL, SPRINT_MUL, WALKSPEED},
    definition::PlayerInputs,
};
use crate::{
    inventory::{
        config::{FLY_SPEED, FLY_SPRINT_MUL},
        definition::Flying,
    },
    net::config::TIMESTEP,
    world::chunk::{ChunkIndex, ChunkPos},
};

/// Moves every player matching `F` by their inputs. The server runs it for all
/// players (`With<PlayerMarker>`), the client for the ones it predicts
/// (`With<Predicted>`), so both sides share one movement system.
///
/// A player whose surroundings haven't loaded yet is held still, without
/// gravity, like Minecraft does. Otherwise they'd walk or fall into a chunk that
/// doesn't exist yet (no collider), and end up inside its ground once it appears.
/// "Loaded" means whatever this side has: generated chunks on the server,
/// received ones on the client.
#[allow(clippy::type_complexity)] // a single-use query reads worse as an alias
pub fn player_motion<F: QueryFilter + 'static>(
    mut players: Query<
        (
            &Position,
            &mut Rotation,
            &mut LinearVelocity,
            &mut GravityScale,
            &ShapeHits,
            &ActionState<PlayerInputs>,
            Option<&Flying>,
        ),
        F,
    >,
    index: Res<ChunkIndex>,
) {
    for (position, mut rot, mut vel, mut gravity, hits, action, flying) in &mut players {
        let loaded = surroundings_loaded(&index, position.0);
        let flying = flying.is_some_and(|flying| flying.0);
        // no gravity while waiting for the world or flying. Only written when it
        // changes, so avian isn't told it changed every tick
        let scale = if loaded && !flying { 1.0 } else { 0.0 };
        if gravity.0 != scale {
            gravity.0 = scale;
        }
        if !loaded {
            vel.0 = Vec3::ZERO;
            continue;
        }
        if flying {
            fly(&mut rot, &mut vel, &action.0);
        } else {
            apply_input(&mut rot, &mut vel, &action.0, !hits.is_empty());
        }
    }
}

/// Flying: move straight where the inputs say, no gravity, no momentum. Jump
/// goes up, crouch goes down
pub fn fly(rotation: &mut Rotation, velocity: &mut LinearVelocity, input: &PlayerInputs) {
    let look_x = Quat::from_rotation_y(input.look.x);
    *rotation = Rotation(look_x);
    let speed = FLY_SPEED * if input.sprint { FLY_SPRINT_MUL } else { 1.0 };
    let up = f32::from(input.jump) - f32::from(input.descend);
    let wish = look_x * Vec3::new(input.motion.x, 0.0, input.motion.y).clamp_length_max(1.0);
    velocity.0 = (wish + Vec3::Y * up) * speed;
}

/// Whether the chunk around `position` and all 26 chunks touching it exist, so
/// there's ground wherever the player can get to in the next moments
fn surroundings_loaded(index: &ChunkIndex, position: Vec3) -> bool {
    let center = ChunkPos::containing(position).0;
    (-1..=1).all(|y| {
        (-1..=1).all(|z| (-1..=1).all(|x| index.contains(ChunkPos(center + IVec3::new(x, y, z)))))
    })
}

/// Hosts agreed-upon changes between client and server about how motion inputs
/// are processed. It is essential that there are no disagreements between
/// motion as it will cause client-side rubberbanding.
pub fn apply_input(
    rotation: &mut Rotation,
    velocity: &mut LinearVelocity,
    input: &PlayerInputs,
    grounded: bool, // grounded must be defined by user: `!hits.is_empty()`
) {
    // direction player is looking
    let look_x = Quat::from_rotation_y(input.look.x);
    // TODO: Allow the camera to look around, but only rotate the body on yaw
    *rotation = Rotation(look_x);

    // `PlayerInputs` is responsible for declaring which is forward
    let speed = WALKSPEED * if input.sprint { SPRINT_MUL } else { 1.0 };
    let wish =
        look_x * Vec3::new(input.motion.x, 0.0, input.motion.y).clamp_length_max(1.0) * speed;

    if grounded {
        velocity.0.x = wish.x;
        velocity.0.z = wish.z;
        if input.jump {
            velocity.0.y = JUMP_VEL;
        }
    } else {
        air_strafe(&mut velocity.0, wish);
    }
}

fn air_strafe(velocity: &mut Vec3, wish: Vec3) {
    let Some(dir) = wish.try_normalize() else {
        return;
    };
    let along = velocity.dot(dir);
    let add = (wish.length() - along).clamp(0.0, AIR_ACCEL * TIMESTEP as f32);
    *velocity += dir * add;
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::world::chunk::{pos, Chunk};

    /// A world with a player at the origin, moving forward, and the chunks at
    /// `loaded` offsets from the player's chunk
    fn world_with(loaded: impl Iterator<Item = IVec3>) -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<ChunkIndex>();
        world.add_observer(pos::index_chunk);
        for offset in loaded {
            world.spawn((ChunkPos(offset), Chunk::default()));
        }
        let input = PlayerInputs {
            motion: Vec2::new(0.0, -1.0),
            ..default()
        };
        let player = world
            .spawn((
                Position(Vec3::new(1.0, 1.0, 1.0)),
                Rotation::default(),
                LinearVelocity(Vec3::new(0.0, -3.0, 0.0)),
                GravityScale(1.0),
                ShapeHits::default(),
                ActionState(input),
            ))
            .id();
        (world, player)
    }

    fn all_around() -> impl Iterator<Item = IVec3> {
        (-1..=1).flat_map(|y| (-1..=1).flat_map(move |z| (-1..=1).map(move |x| IVec3::new(x, y, z))))
    }

    #[test]
    fn players_wait_for_the_world_around_them() {
        // one chunk missing (the one below): held still, no gravity
        let (mut world, player) = world_with(all_around().filter(|&o| o != IVec3::NEG_Y));
        world.run_system_once(player_motion::<()>).unwrap();
        assert_eq!(world.get::<LinearVelocity>(player).unwrap().0, Vec3::ZERO);
        assert_eq!(world.get::<GravityScale>(player).unwrap().0, 0.0);

        // everything loaded: gravity back on, and the input moves them
        let (mut world, player) = world_with(all_around());
        world.run_system_once(player_motion::<()>).unwrap();
        assert_ne!(world.get::<LinearVelocity>(player).unwrap().0, Vec3::ZERO);
        assert_eq!(world.get::<GravityScale>(player).unwrap().0, 1.0);
    }
}
