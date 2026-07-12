use bevy::prelude::*;

use crate::projectile::protocol::PlayerProjectile;

pub const PROJECTILE_SPEED_PER_TICK: f32 = 20.0;
pub const PROJECTILE_LIFETIME_TICKS: u16 = 90;
pub const PROJECTILE_RADIUS: f32 = 8.0;
pub const PROJECTILE_SPAWN_OFFSET: f32 = 25.0 + PROJECTILE_RADIUS + 2.0;

#[derive(Component, Clone, Copy, Debug, PartialEq, Deref, DerefMut)]
pub struct ProjectilePosition(pub Vec2);

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ProjectileLifetime {
    pub remaining_ticks: u16,
}

#[derive(Component)]
pub struct ServerProjectile;

#[derive(Component)]
pub struct ClientProjectileVisual;

pub fn projectile_spawn_position(player_position: Vec2, direction: Vec2) -> Vec2 {
    player_position + direction.normalize_or_zero() * PROJECTILE_SPAWN_OFFSET
}

pub fn move_projectile(position: &mut ProjectilePosition, projectile: &PlayerProjectile) {
    position.0 += projectile.direction * projectile.speed_per_tick;
}
