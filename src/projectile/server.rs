use bevy::prelude::*;
use lightyear::{
    connection::network_target::NetworkTarget,
    core::timeline::LocalTimeline,
    prediction::despawn::PredictionDespawnCommandsExt,
    prelude::Replicate,
};

use crate::{
    app::ServerState,
    enemy::{EnemyHealth, EnemyPosition},
    fragment::api::SpawnFragmentPool,
    player::api::AddKillsToAristeia,
    player::protocol::{PlayerAristeia, PlayerHealth},
    player::{PlayerId, PlayerPosition},
    projectile::protocol::{PlayerProjectile, ProjectileImpact, ProjectilePosition},
    projectile::shared::{check_projectile_range, move_projectile},
    protocol::rooms::GameRoom,
    settings::GameSettings,
    shared::FixedGameplaySet,
    world::api::AddGlobalAristeia,
};

pub struct ProjectileServerPlugin;

impl Plugin for ProjectileServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (simulate_server_projectiles, expire_server_impacts)
                .chain()
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ImpactLifetime {
    pub remaining_ticks: u16,
}

/// Simulates authoritative projectile motion, hit detection, damage, and death.
pub fn simulate_server_projectiles(
    mut commands: Commands,
    mut projectiles: Query<(
        Entity,
        &PlayerProjectile,
        &mut ProjectilePosition,
        &GameRoom,
    )>,
    mut players: Query<(
        Entity,
        &PlayerId,
        &PlayerPosition,
        &GameRoom,
        &mut PlayerHealth,
        &PlayerAristeia,
    )>,
    mut enemies: Query<(Entity, &EnemyPosition, &GameRoom, &mut EnemyHealth)>,
    local_timeline: Res<LocalTimeline>,
    mut fragment_drops: MessageWriter<SpawnFragmentPool>,
    mut aristeia_tracking: MessageWriter<AddKillsToAristeia>,
    mut global_aristeia: MessageWriter<AddGlobalAristeia>,
    settings: Res<GameSettings>,
) {
    for (projectile_entity, projectile, mut position, room) in &mut projectiles {
        move_projectile(&mut position, projectile);

        if get_hit_player(&mut players, projectile, &position, room, &settings).is_some() {
            spawn_projectile_impact(
                &mut commands,
                projectile_entity,
                projectile,
                &position,
                &settings,
            );
            continue;
        }

        if let Some((enemy_entity, enemy_died)) =
            get_hit_enemy(&mut enemies, projectile, &position, room, &settings)
        {
            spawn_projectile_impact(
                &mut commands,
                projectile_entity,
                projectile,
                &position,
                &settings,
            );
            if enemy_died {
                let personal_aristeia = players
                    .iter()
                    .find(|(_, player_id, _, _, _, _)| player_id.0 == projectile.owner)
                    .map(|(_, _, _, _, _, aristeia)| aristeia.current)
                    .unwrap_or(0);
                let bonus = u32::from(settings.fragment.bonus_drops_per_aristeia)
                    .saturating_mul(personal_aristeia);
                let drop_count = u32::from(settings.fragment.base_drop_count)
                    .saturating_add(bonus)
                    .min(u32::from(settings.fragment.maximum_drop_count))
                    as u16;

                fragment_drops.write(SpawnFragmentPool::new(position.0, drop_count, *room));
                commands.entity(enemy_entity).despawn();
                aristeia_tracking.write(AddKillsToAristeia::new(projectile.owner, 1));
                global_aristeia.write(AddGlobalAristeia(
                    settings.global_aristeia.contribution_per_kill,
                ));
            }
            continue;
        }

        check_projectile_range(
            &mut commands,
            &local_timeline,
            projectile_entity,
            projectile,
            &position,
            room,
            &settings,
        );
    }
}

fn spawn_projectile_impact(
    commands: &mut Commands<'_, '_>,
    projectile_entity: Entity,
    projectile: &PlayerProjectile,
    position: &ProjectilePosition,
    settings: &GameSettings,
) {
    commands.spawn((
        ProjectileImpact {
            position: position.0,
            weapon: projectile.weapon,
            damage: projectile.damage,
        },
        ImpactLifetime {
            remaining_ticks: settings.projectile.impact_lifetime_ticks,
        },
        Replicate::to_clients(NetworkTarget::All),
        Name::new("Server Projectile Impact"),
    ));

    commands.entity(projectile_entity).prediction_despawn();
}

fn get_hit_player(
    players: &mut Query<
        '_,
        '_,
        (
            Entity,
            &PlayerId,
            &PlayerPosition,
            &GameRoom,
            &mut PlayerHealth,
            &PlayerAristeia,
        ),
    >,
    projectile: &PlayerProjectile,
    position: &ProjectilePosition,
    projectile_room: &GameRoom,
    settings: &GameSettings,
) -> Option<Entity> {
    for (player_entity, player_id, player_position, player_room, mut health, _) in players {
        if player_id.0 == projectile.owner || player_room != projectile_room || health.current == 0
        {
            continue;
        }

        let collision_radius = projectile.radius + settings.player.collision_radius;
        let hit =
            position.0.distance_squared(player_position.0) <= collision_radius * collision_radius;

        if !hit {
            continue;
        }

        health.current = health.current.saturating_sub(projectile.damage);

        info!(
            ?player_entity,
            current_health = health.current,
            maximum_health = health.maximum,
            "Player took projectile damage"
        );

        return Some(player_entity);
    }

    None
}

fn get_hit_enemy(
    enemies: &mut Query<'_, '_, (Entity, &EnemyPosition, &GameRoom, &mut EnemyHealth)>,
    projectile: &PlayerProjectile,
    position: &ProjectilePosition,
    projectile_room: &GameRoom,
    settings: &GameSettings,
) -> Option<(Entity, bool)> {
    for (enemy_entity, enemy_position, enemy_room, mut health) in enemies {
        if enemy_room != projectile_room || health.current == 0 {
            continue;
        }

        let collision_radius = projectile.radius + settings.enemy.collision_radius;
        let hit =
            position.0.distance_squared(enemy_position.0) <= collision_radius * collision_radius;

        if !hit {
            continue;
        }

        health.current = health.current.saturating_sub(projectile.damage);
        return Some((enemy_entity, health.current == 0));
    }

    None
}

pub fn expire_server_impacts(
    mut commands: Commands,
    mut impacts: Query<(Entity, &mut ImpactLifetime)>,
) {
    for (entity, mut lifetime) in &mut impacts {
        lifetime.remaining_ticks = lifetime.remaining_ticks.saturating_sub(1);

        if lifetime.remaining_ticks == 0 {
            commands.entity(entity).despawn();
        }
    }
}
