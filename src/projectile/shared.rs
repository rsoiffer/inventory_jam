use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::{
    enemy::protocol::{ENEMY_HALF_SIZE, ENEMY_SIZE, EnemyPosition},
    projectile::protocol::PlayerProjectile,
};

pub const PROJECTILE_SPEED_PER_TICK: f32 = 20.0;
pub const PROJECTILE_LIFETIME_TICKS: u16 = 90;
pub const PROJECTILE_RADIUS: f32 = 8.0;
pub const PROJECTILE_SPAWN_OFFSET: f32 = 25.0 + PROJECTILE_RADIUS + 2.0;

const COLLISION_CELL_SIZE: f32 = ENEMY_SIZE;

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

/// Checks for collisions between projectiles and enemies on both the client and server
pub fn projectile_collision_system(
    mut commands: Commands,
    projectiles: Query<(
        Entity,
        &ProjectilePosition,
        Has<ServerProjectile>,
        Has<ClientProjectileVisual>,
    )>,
    enemies: Query<(Entity, &EnemyPosition)>,
) {
    let mut cells: HashMap<(i32, i32), Vec<(Entity, Vec2)>> = HashMap::new();

    for (entity, position) in &enemies {
        cells
            .entry(position_to_cell(position.0))
            .or_default()
            .push((entity, position.0));
    }

    let mut hit_enemies = HashSet::new();

    for (projectile_entity, projectile_position, is_server, is_client_visual) in &projectiles {
        let cell = position_to_cell(projectile_position.0);
        let mut hit_enemy = None;

        'neighbors: for dx in -1..=1 {
            for dy in -1..=1 {
                let Some(enemies_in_cell) = cells.get(&(cell.0 + dx, cell.1 + dy)) else {
                    continue;
                };

                for &(enemy_entity, enemy_position) in enemies_in_cell {
                    if hit_enemies.contains(&enemy_entity) {
                        continue;
                    }

                    if circle_intersects_aabb(
                        projectile_position.0,
                        PROJECTILE_RADIUS,
                        enemy_position,
                        ENEMY_HALF_SIZE,
                    ) {
                        hit_enemy = Some(enemy_entity);
                        break 'neighbors;
                    }
                }
            }
        }

        let Some(enemy_entity) = hit_enemy else {
            continue;
        };

        hit_enemies.insert(enemy_entity);

        if is_server {
            commands.entity(projectile_entity).despawn();
            commands.entity(enemy_entity).despawn();
        } else if is_client_visual {
            // Do nothing for now, in the future predict a despawn
        }
    }
}

fn position_to_cell(position: Vec2) -> (i32, i32) {
    (
        (position.x / COLLISION_CELL_SIZE).floor() as i32,
        (position.y / COLLISION_CELL_SIZE).floor() as i32,
    )
}

fn circle_intersects_aabb(
    circle_center: Vec2,
    radius: f32,
    aabb_center: Vec2,
    half_extents: f32,
) -> bool {
    let closest = circle_center.clamp(
        aabb_center - Vec2::splat(half_extents),
        aabb_center + Vec2::splat(half_extents),
    );
    circle_center.distance_squared(closest) <= radius * radius
}
