use bevy::prelude::*;
use lightyear::prelude::*;

use crate::app::AppState;
use crate::enemy::{EnemyHealth, EnemyPosition};
use crate::protocol::rooms::GameRoom;
use crate::shared::FixedGameplaySet;
use crate::{
    enemy::shared::ENEMY_COLLISION_RADIUS,
    projectile::{
        protocol::{PlayerProjectile, ProjectileImpact},
        shared::{
            self as projectile_shared, ImpactLifetime, ProjectileLifetime, ProjectilePosition,
            ServerImpact, ServerProjectile,
        },
    },
};

pub struct ProjectileServerPlugin;

impl Plugin for ProjectileServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (simulate_server_projectiles, expire_server_impacts)
                .chain()
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(AppState::Hosting)),
        );
    }
}

/// Simulates authoritative projectile motion, hit detection, damage, and death.
fn simulate_server_projectiles(
    mut commands: Commands,
    mut projectiles: Query<
        (
            Entity,
            &PlayerProjectile,
            &mut ProjectilePosition,
            &mut ProjectileLifetime,
            &GameRoom,
        ),
        With<ServerProjectile>,
    >,
    mut enemies: Query<(Entity, &EnemyPosition, &mut EnemyHealth)>,
) {
    for (projectile_entity, projectile, mut position, mut lifetime, room) in &mut projectiles {
        projectile_shared::move_projectile(&mut position, projectile);

        let mut hit_enemy = None;

        for (enemy_entity, enemy_position, mut health) in &mut enemies {
            if health.current == 0 {
                continue;
            }

            let collision_radius = projectile.radius + ENEMY_COLLISION_RADIUS;
            let hit = position.0.distance_squared(enemy_position.0)
                <= collision_radius * collision_radius;

            if !hit {
                continue;
            }

            health.current = health.current.saturating_sub(projectile.damage);
            hit_enemy = Some((enemy_entity, health.current == 0));
            break;
        }

        if let Some((enemy_entity, enemy_died)) = hit_enemy {
            commands.spawn((
                ProjectileImpact {
                    position: position.0,
                    weapon: projectile.weapon,
                    damage: projectile.damage,
                },
                ImpactLifetime {
                    remaining_ticks: projectile_shared::IMPACT_LIFETIME_TICKS,
                },
                ServerImpact,
                Replicate::to_clients(NetworkTarget::All),
                Name::new("Server Projectile Impact"),
            ));

            commands.entity(projectile_entity).despawn();

            if enemy_died {
                commands.entity(enemy_entity).despawn();
            }

            continue;
        }

        lifetime.remaining_ticks = lifetime.remaining_ticks.saturating_sub(1);

        if lifetime.remaining_ticks == 0
            || projectile_shared::projectile_reached_max_range(*position, projectile)
            || projectile_is_outside_world(position.0, room)
        {
            commands.entity(projectile_entity).despawn();
        }
    }
}

fn expire_server_impacts(
    mut commands: Commands,
    mut impacts: Query<(Entity, &mut ImpactLifetime), With<ServerImpact>>,
) {
    for (entity, mut lifetime) in &mut impacts {
        lifetime.remaining_ticks = lifetime.remaining_ticks.saturating_sub(1);

        if lifetime.remaining_ticks == 0 {
            commands.entity(entity).despawn();
        }
    }
}

fn projectile_is_outside_world(position: Vec2, room: &GameRoom) -> bool {
    !room.bounds().contains(position)
}
