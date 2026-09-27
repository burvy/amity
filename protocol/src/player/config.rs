use crate::world::config::BLOCK_SIZE;

/// Shared walkspeed to ensure similar simulation between client and server
pub const WALKSPEED: f32 = 3.0;
/// Shared sprint speed multiplier
pub const SPRINT_MUL: f32 = 1.5;

/// How fast a player can change their velocity mid-air (metres per second, per second)
pub const AIR_ACCEL: f32 = 1.0;

/// Shared jump velocity to ensure client-server similarity
pub const JUMP_VEL: f32 = 4.5;

/// Shared player radius for simulation similarity
pub const PLAYER_RADIUS: f32 = 0.3;
/// Shared player length for simulation similarity
/// This describes the cylinder section of the player,
/// excluding the caps. The total height is:
/// PLAYER_LENGTH + (2.0 * PLAYER_RADIUS) which is 1.8m
/// Recalculate for the doc if anything changes with these two constants
pub const PLAYER_LENGTH: f32 = 1.2;

/// Eye height for the camera for client or possible server utilities
pub const EYE_HEIGHT: f32 = 0.7;

/// Distance from player center to the top of the capsule (don't change)
pub const PLAYER_HALF_HEIGHT: f32 = PLAYER_LENGTH / 2.0 + PLAYER_RADIUS;

/// Distance to the ground to be considered grounded (tweak if needed)
pub const GROUND_TOLERANCE: f32 = 0.1;

/// Distance to consider interaction
pub const INTERACT_DISTANCE: f32 = 4.0;

/// Highest ledge a player walks onto without jumping: one block, plus a margin
pub const STEP_HEIGHT: f32 = BLOCK_SIZE + 0.05;
/// Step casts start this far above the body so the ground it rests on doesn't
/// count as an obstacle
pub const STEP_SKIN: f32 = 0.02;
