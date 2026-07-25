use bevy::prelude::*;
use lightyear::{
    core::{tick::TickDuration, timeline::LocalTimeline},
    prediction::Predicted,
};
use rand::Rng;

use crate::{
    app::ServerState,
    enemy::{EnemyHealth, EnemyKind, EnemyPosition, server::EnemyStats},
    fragment::api::SpawnFragmentPool,
    persistence::CachedPersistentState,
    player::PlayerId,
    player::PlayerPosition,
    player::api::AddKillsToAristeia,
    player::protocol::{PlayerAristeia, PlayerHealth},
    player::server::PlayerDeathTimer,
    projectile::protocol::{self, ProjectileBuffer, ProjectileSource},
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
            (
                simulate_player_projectile_buffers,
                simulate_enemy_projectile_buffers,
            )
                .chain()
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

fn simulate_player_projectile_buffers(
    mut commands: Commands,
    mut players: Query<(
        Has<Predicted>,
        &PlayerId,
        &GameRoom,
        &mut ProjectileBuffer,
        &PlayerAristeia,
        &CachedPersistentState,
    )>,
    mut enemies: Query<(
        Entity,
        &EnemyPosition,
        &EnemyKind,
        &GameRoom,
        &mut EnemyHealth,
        &EnemyStats,
    )>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    mut fragment_drops: MessageWriter<SpawnFragmentPool>,
    mut aristeia_tracking: MessageWriter<AddKillsToAristeia>,
    mut global_aristeia: MessageWriter<AddGlobalAristeia>,
    settings: Res<GameSettings>,
) {
    let skip_predicted = !host_server.is_empty();
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();
    let mut rng = rand::rng();

    for (predicted, owner_id, room, mut buffer, aristeia, cache) in &mut players {
        if skip_predicted && predicted {
            continue;
        }

        let owner = owner_id.0;
        let personal_aristeia = aristeia.current;
        let progress = cache.weapon(cache.equipped_weapon_id);
        let damage_level = progress.level;
        let crit_chance = settings.progression.crit_chance_at(progress.crit_level);
        buffer.simulate(room, tick, tick_secs, &settings, |source, pos, last_hit| {
            let ProjectileSource::Weapon(weapon) = source else {
                return None;
            };
            let stats = settings.weapons.get(weapon)?;
            let mut damage = settings.progression.damage_at(stats.damage, damage_level);
            if rng.random::<f32>() < crit_chance {
                damage =
                    (damage as f32 * settings.progression.crit_damage_multiplier).round() as u32;
            }
            let radius = stats.projectile_radius;

            for (entity, enemy, kind, enemy_room, mut health, enemy_stats) in &mut enemies {
                if enemy_room != room
                    || health.current == 0
                    || entity.to_bits() == last_hit
                    || !protocol::overlaps(
                        pos,
                        enemy.0,
                        radius,
                        kind.collision_radius(settings.enemy.collision_radius),
                    )
                {
                    continue;
                }
                health.current = health.current.saturating_sub(damage);
                if health.current == 0 {
                    let bonus = u32::from(settings.fragment.bonus_drops_per_aristeia)
                        .saturating_mul(personal_aristeia);
                    let base = u32::from(settings.fragment.base_drop_count)
                        .saturating_add(bonus)
                        .min(u32::from(settings.fragment.maximum_drop_count));
                    let total_value =
                        ((base as f32) * enemy_stats.fragment_multiplier).round() as u32;
                    fragment_drops.write(SpawnFragmentPool::new(pos, total_value.max(1), *room));
                    commands.entity(entity).despawn();
                    aristeia_tracking.write(AddKillsToAristeia::new(owner, 1));
                    global_aristeia.write(AddGlobalAristeia(
                        settings.global_aristeia.contribution_per_kill,
                    ));
                }
                return Some(entity.to_bits());
            }

            None
        });
    }
}

fn simulate_enemy_projectile_buffers(
    mut players: Query<(
        Entity,
        &PlayerPosition,
        &GameRoom,
        &mut PlayerHealth,
        &mut PlayerDeathTimer,
    )>,
    mut enemies: Query<(&GameRoom, &mut ProjectileBuffer, &EnemyStats), Without<PlayerId>>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();
    let radius = settings.enemy.projectile_radius;
    let player_targets: Vec<(Entity, Vec2, GameRoom, bool)> = players
        .iter()
        .map(|(entity, position, room, health, _)| (entity, position.0, *room, health.current > 0))
        .collect();
    let mut pending_hits: Vec<(Entity, u32)> = Vec::new();

    for (room, mut buffer, stats) in &mut enemies {
        let damage = stats.ranged_damage;
        if damage == 0 {
            buffer.simulate(room, tick, tick_secs, &settings, |_, _, _| None);
            continue;
        }
        buffer.simulate(room, tick, tick_secs, &settings, |source, pos, last_hit| {
            if source != ProjectileSource::Enemy {
                return None;
            }
            for (player_entity, player_position, player_room, alive) in &player_targets {
                if player_room != room
                    || !*alive
                    || player_entity.to_bits() == last_hit
                    || !protocol::overlaps(
                        pos,
                        *player_position,
                        radius,
                        settings.player.collision_radius,
                    )
                {
                    continue;
                }
                pending_hits.push((*player_entity, damage));
                return Some(player_entity.to_bits());
            }
            None
        });
    }

    for (player_entity, damage) in pending_hits {
        let Ok((_, _, _, mut health, mut death_timer)) = players.get_mut(player_entity) else {
            continue;
        };
        if health.current == 0 {
            continue;
        }
        health.current = health.current.saturating_sub(damage);
        if health.current == 0 {
            death_timer.remaining_seconds = settings.player.death_screen_duration_seconds;
        }
    }
}
