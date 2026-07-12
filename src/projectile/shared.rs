use bevy::prelude::*;

use crate::projectile::protocol::PlayerProjectile;

#[derive(Component, Clone, Copy, Debug, PartialEq, Deref, DerefMut)]
pub struct ProjectilePosition(pub Vec2);

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ProjectileLifetime {
    pub remaining_ticks: u16,
}

impl ProjectileLifetime {
    pub fn from_projectile(projectile: &PlayerProjectile) -> Self {
        let travel_ticks =
            (projectile.max_range / projectile.speed_per_tick.max(0.001)).ceil() as u32;

        Self {
            remaining_ticks: travel_ticks.saturating_add(2).min(u16::MAX as u32) as u16,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ImpactLifetime {
    pub remaining_ticks: u16,
}

pub const IMPACT_LIFETIME_TICKS: u16 = 12;

#[derive(Component)]
pub struct ServerProjectile;

#[derive(Component)]
pub struct ClientProjectileVisual;

#[derive(Component)]
pub struct ServerImpact;

#[derive(Component)]
pub struct ClientImpactVisual;

#[derive(Component, Clone, Copy, Debug, PartialEq, Deref, DerefMut)]
pub struct ImpactPosition(pub Vec2);

pub fn projectile_spawn_position(
    player_position: Vec2,
    direction: Vec2,
    projectile_radius: f32,
) -> Vec2 {
    const PLAYER_HALF_SIZE: f32 = 25.0;
    const SPAWN_GAP: f32 = 2.0;

    player_position
        + direction.normalize_or_zero()
            * (PLAYER_HALF_SIZE + projectile_radius.max(0.0) + SPAWN_GAP)
}

pub fn move_projectile(position: &mut ProjectilePosition, projectile: &PlayerProjectile) {
    position.0 += projectile.direction.normalize_or_zero() * projectile.speed_per_tick;
}

pub fn projectile_reached_max_range(
    position: ProjectilePosition,
    projectile: &PlayerProjectile,
) -> bool {
    position.0.distance_squared(projectile.origin) >= projectile.max_range * projectile.max_range
}
