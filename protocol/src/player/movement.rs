//! Walking, running, jumping, falling and flying.
//!
//! Players are kinematic boxes (`definition::PlayerBody`): each tick this works
//! out the velocity a player wants, then sweeps their box along it with avian's
//! move-and-slide, which stops at whatever the box hits and slides along it. The
//! velocity left after sliding is kept for the next tick, so a player who lands
//! loses their fall speed and one who walks into a wall stops pushing into it.

use core::time::Duration;

use avian3d::prelude::*;
use bevy::{ecs::query::QueryFilter, prelude::*};
use lightyear::prelude::input::native::ActionState;

use super::{
    config::{
        AIR_ACCEL, GROUND_ACCEL, GROUND_NORMAL_MIN, GROUND_PROBE, JUMP_HEIGHT, RUN_SPEED,
        TERMINAL_SPEED, WALK_SPEED,
    },
    definition::{player_collider, PlayerInputs},
};
use crate::{
    inventory::{
        config::{FLY_SPEED, FLY_SPRINT_MUL},
        definition::Flying,
    },
    net::config::TIMESTEP,
    world::chunk::{ChunkIndex, ChunkPos},
};

/// One player's state, read out before anyone moves
struct Mover {
    entity: Entity,
    position: Vec3,
    velocity: Vec3,
    input: PlayerInputs,
    flying: bool,
}

/// Moves every player matching `F` by their inputs. The server runs it for all
/// players (`With<PlayerMarker>`), the client for the ones it predicts
/// (`With<Predicted>`), so both sides share one movement system.
///
/// A player whose surroundings haven't loaded yet is held still, like
/// Minecraft does. Otherwise they'd walk or fall into a chunk that doesn't exist
/// yet (no collider), and end up inside its ground once it appears. "Loaded"
/// means whatever this side has: generated chunks on the server, received ones
/// on the client.
///
/// Reads every player first, then moves them, then writes them back:
/// move-and-slide reads every collider's `Position`, players' included, so the
/// two can't be borrowed at once. Everyone moves against where the others were
/// at the start of the tick, so the order players are visited in doesn't matter.
#[allow(clippy::type_complexity)] // a ParamSet of queries reads worse as an alias
pub fn player_motion<F: QueryFilter + 'static>(
    mut set: ParamSet<(
        Query<
            (
                Entity,
                &mut Position,
                &mut LinearVelocity,
                &ActionState<PlayerInputs>,
                Option<&Flying>,
            ),
            F,
        >,
        MoveAndSlide,
    )>,
    index: Res<ChunkIndex>,
    gravity: Res<Gravity>,
) {
    let movers: Vec<Mover> = set
        .p0()
        .iter()
        .map(|(entity, position, velocity, action, flying)| Mover {
            entity,
            position: position.0,
            velocity: velocity.0,
            input: action.0.clone(),
            flying: flying.is_some_and(|flying| flying.0),
        })
        .collect();

    let gravity = -gravity.0.y;
    let moved: Vec<(Entity, Vec3, Vec3)> = {
        let slide = set.p1();
        let body = player_collider();
        let config = MoveAndSlideConfig::default();
        movers
            .into_iter()
            .map(|mover| {
                if !surroundings_loaded(&index, mover.position) {
                    return (mover.entity, mover.position, Vec3::ZERO);
                }
                let filter = SpatialQueryFilter::from_excluded_entities([mover.entity]);
                let wanted = if mover.flying {
                    fly(&mover.input)
                } else {
                    let grounded = grounded(&slide, &body, mover.position, &filter);
                    walk(mover.velocity, &mover.input, grounded, gravity)
                };
                let out = slide.move_and_slide(
                    &body,
                    mover.position,
                    Quat::IDENTITY,
                    wanted,
                    Duration::from_secs_f64(TIMESTEP),
                    &config,
                    &filter,
                    |_| MoveAndSlideHitResponse::Accept,
                );
                (mover.entity, out.position, out.projected_velocity)
            })
            .collect()
    };

    let mut players = set.p0();
    for (entity, position, velocity) in moved {
        let Ok((_, mut pos, mut vel, _, _)) = players.get_mut(entity) else {
            continue;
        };
        // written only when changed, so a player standing still isn't marked
        // as moved every tick
        pos.set_if_neq(Position(position));
        vel.set_if_neq(LinearVelocity(velocity));
    }
}

/// Whether the box at `position` stands on something: any part of its bottom
/// resting on a surface flat enough to be ground. Sweeps the whole box down a
/// hair, so a player on the very edge of a block still counts
fn grounded(
    slide: &MoveAndSlide,
    body: &Collider,
    position: Vec3,
    filter: &SpatialQueryFilter,
) -> bool {
    let skin = MoveAndSlideConfig::default().skin_width;
    slide
        .cast_move(
            body,
            position,
            Quat::IDENTITY,
            Vec3::NEG_Y * GROUND_PROBE,
            skin,
            filter,
        )
        .is_some_and(|hit| hit.normal1.y >= GROUND_NORMAL_MIN)
}

/// The horizontal velocity the inputs ask for: the move keys, turned to face
/// where the player looks, at walking or running speed
fn wish_velocity(input: &PlayerInputs) -> Vec3 {
    let look_x = Quat::from_rotation_y(input.look.x);
    let speed = if input.sprint { RUN_SPEED } else { WALK_SPEED };
    // `PlayerInputs` is responsible for declaring which is forward
    look_x * Vec3::new(input.motion.x, 0.0, input.motion.y).clamp_length_max(1.0) * speed
}

/// Walking and falling: the velocity to move at this tick, from last tick's.
///
/// On the ground the player eases toward the speed they want, and can jump.
/// In the air they keep their speed and only steer a little. Gravity always
/// pulls; on the ground, move-and-slide takes that back off when the box meets
/// the floor. `gravity` is its strength (metres per second, per second).
pub fn walk(velocity: Vec3, input: &PlayerInputs, grounded: bool, gravity: f32) -> Vec3 {
    let dt = TIMESTEP as f32;
    let wish = wish_velocity(input);
    let horizontal = velocity.with_y(0.0);
    let horizontal = if grounded {
        horizontal.move_towards(wish, GROUND_ACCEL * dt)
    } else {
        air_strafe(horizontal, wish)
    };

    let mut vertical = velocity.y;
    if grounded && input.jump {
        vertical = jump_speed(gravity);
    }
    vertical = (vertical - gravity * dt).max(-TERMINAL_SPEED);
    horizontal.with_y(vertical)
}

/// The upward speed that lifts the body `JUMP_HEIGHT`. A throw straight up
/// rises v^2 / 2g; moving in whole ticks loses about half a tick of it (the
/// position steps with the speed after gravity takes its bit), which the extra
/// g*dt/2 gives back
fn jump_speed(gravity: f32) -> f32 {
    (2.0 * gravity * JUMP_HEIGHT).sqrt() + gravity * TIMESTEP as f32 / 2.0
}

/// Mid-air steering: pushes toward the wished direction, but never past the
/// wished speed along it, and never slows the player. A jump keeps its speed
/// with no keys held
fn air_strafe(velocity: Vec3, wish: Vec3) -> Vec3 {
    let Some(dir) = wish.try_normalize() else {
        return velocity;
    };
    let along = velocity.dot(dir);
    let add = (wish.length() - along).clamp(0.0, AIR_ACCEL * TIMESTEP as f32);
    velocity + dir * add
}

/// Flying: move straight where the inputs say, no gravity, no momentum. Jump
/// goes up, crouch goes down
pub fn fly(input: &PlayerInputs) -> Vec3 {
    let look_x = Quat::from_rotation_y(input.look.x);
    let speed = FLY_SPEED * if input.sprint { FLY_SPRINT_MUL } else { 1.0 };
    let up = f32::from(input.jump) - f32::from(input.descend);
    let wish = look_x * Vec3::new(input.motion.x, 0.0, input.motion.y).clamp_length_max(1.0);
    (wish + Vec3::Y * up) * speed
}

/// Whether the chunk around `position` and all 26 chunks touching it exist, so
/// there's ground wherever the player can get to in the next moments
fn surroundings_loaded(index: &ChunkIndex, position: Vec3) -> bool {
    let center = ChunkPos::containing(position).0;
    (-1..=1).all(|y| {
        (-1..=1).all(|z| (-1..=1).all(|x| index.contains(ChunkPos(center + IVec3::new(x, y, z)))))
    })
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::world::chunk::{pos, Chunk};

    const G: f32 = 9.81;

    fn forward() -> PlayerInputs {
        PlayerInputs {
            motion: Vec2::new(0.0, -1.0),
            ..default()
        }
    }

    /// A world with physics, a player at the origin moving forward, and the
    /// chunks at `loaded` offsets from the player's chunk (empty, so nothing
    /// to collide with)
    fn world_with(loaded: impl Iterator<Item = IVec3>) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PhysicsPlugins::default()));
        app.finish();
        app.cleanup();
        let world = app.world_mut();
        world.init_resource::<ChunkIndex>();
        world.add_observer(pos::index_chunk);
        for offset in loaded {
            world.spawn((ChunkPos(offset), Chunk::default()));
        }
        let player = world
            .spawn((
                Position(Vec3::new(1.0, 1.0, 1.0)),
                LinearVelocity(Vec3::new(0.0, -3.0, 0.0)),
                ActionState(forward()),
            ))
            .id();
        (app, player)
    }

    fn all_around() -> impl Iterator<Item = IVec3> {
        (-1..=1)
            .flat_map(|y| (-1..=1).flat_map(move |z| (-1..=1).map(move |x| IVec3::new(x, y, z))))
    }

    #[test]
    fn players_wait_for_the_world_around_them() {
        // one chunk missing (the one below): held still
        let (mut app, player) = world_with(all_around().filter(|&o| o != IVec3::NEG_Y));
        app.world_mut()
            .run_system_once(player_motion::<()>)
            .unwrap();
        assert_eq!(
            app.world().get::<LinearVelocity>(player).unwrap().0,
            Vec3::ZERO
        );
        assert_eq!(app.world().get::<Position>(player).unwrap().0, Vec3::ONE);

        // everything loaded: the input and gravity move them
        let (mut app, player) = world_with(all_around());
        app.world_mut()
            .run_system_once(player_motion::<()>)
            .unwrap();
        assert_ne!(app.world().get::<Position>(player).unwrap().0, Vec3::ONE);
    }

    #[test]
    fn a_jump_clears_its_height() {
        let jump = PlayerInputs {
            jump: true,
            ..default()
        };
        let (mut height, mut velocity) = (0.0, walk(Vec3::ZERO, &jump, true, G));
        let mut peak = 0.0f32;
        while velocity.y > 0.0 || height > 0.0 {
            height += velocity.y * TIMESTEP as f32;
            peak = peak.max(height);
            velocity = walk(velocity, &PlayerInputs::default(), false, G);
        }
        assert!((peak - JUMP_HEIGHT).abs() < 0.01, "peaked at {peak}");
    }

    #[test]
    fn walking_eases_up_to_speed() {
        let first = walk(Vec3::ZERO, &forward(), true, G).with_y(0.0).length();
        assert!(first > 0.0 && first < WALK_SPEED);
        let mut velocity = Vec3::ZERO;
        for _ in 0..64 {
            velocity = walk(velocity.with_y(0.0), &forward(), true, G);
        }
        assert!((velocity.with_y(0.0).length() - WALK_SPEED).abs() < 1e-4);
    }

    #[test]
    fn the_air_keeps_speed() {
        let running = Vec3::new(RUN_SPEED, 0.0, 0.0);
        let after = walk(running, &PlayerInputs::default(), false, G);
        assert_eq!(after.with_y(0.0), running);
        assert!(after.y < 0.0);
    }
}
