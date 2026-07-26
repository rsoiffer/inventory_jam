use bevy::prelude::*;
use lightyear::core::{tick::TickDuration, timeline::LocalTimeline};
use rand::Rng;

use crate::{
    app::ServerState,
    enemy::{EnemyHealth, EnemyIdentity, EnemyKind, EnemyPosition},
    fragment::api::SpawnFragmentPool,
    persistence::{CachedPersistentState, PersistenceReady, Transaction},
    player::PlayerId,
    player::PlayerPosition,
    player::PlayerUsername,
    player::api::AddAristeiaPoints,
    player::protocol::{PlayerAristeia, PlayerHealth},
    player::server::{PlayerDeathTimer, apply_death_penalty},
    player::shared::apply_damage_to_player,
    projectile::protocol::{self, ProjectileBuffer, ProjectileSource},
    protocol::rooms::GameRoom,
    settings::GameSettings,
    shared::FixedGameplaySet,
    world::api::{AddGlobalAristeia, GrandChampionDefeated},
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
        &PlayerId,
        &GameRoom,
        &mut ProjectileBuffer,
        &PlayerAristeia,
        &CachedPersistentState,
    )>,
    mut enemies: Query<(
        Entity,
        &EnemyPosition,
        &GameRoom,
        &mut EnemyHealth,
        &EnemyIdentity,
    )>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    mut fragment_drops: MessageWriter<SpawnFragmentPool>,
    mut aristeia_tracking: MessageWriter<AddAristeiaPoints>,
    mut global_aristeia: MessageWriter<AddGlobalAristeia>,
    mut boss_defeats: MessageWriter<GrandChampionDefeated>,
    settings: Res<GameSettings>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();
    let mut rng = rand::rng();

    for (owner_id, room, mut buffer, aristeia, cache) in &mut players {
        let owner = owner_id.0;
        let personal_aristeia = aristeia.current;
        let progress = cache.weapon(cache.equipped_weapon_id);
        let damage_level = progress.damage_level;
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

            for (entity, enemy, enemy_room, mut health, identity) in &mut enemies {
                if enemy_room != room
                    || health.current == 0
                    || entity.to_bits() == last_hit
                    || !protocol::overlaps(
                        pos,
                        enemy.0,
                        radius,
                        identity.kind.scale(settings.enemy.collision_radius),
                    )
                {
                    continue;
                }
                health.current = health.current.saturating_sub(damage);
                if health.current == 0 {
                    let stats = identity.stats(&settings);
                    let bonus = u32::from(settings.fragment.bonus_drops_per_aristeia)
                        .saturating_mul(personal_aristeia);
                    let base = u32::from(settings.fragment.base_drop_count)
                        .saturating_add(bonus)
                        .min(u32::from(settings.fragment.maximum_drop_count));
                    let total_value = ((base as f32) * stats.fragment_multiplier).round() as u32;
                    fragment_drops.write(SpawnFragmentPool::new(pos, total_value.max(1)));
                    commands.entity(entity).despawn();
                    aristeia_tracking.write(AddAristeiaPoints::new(
                        owner,
                        stats.personal_aristeia_reward,
                    ));
                    if identity.kind == EnemyKind::GrandChampion {
                        boss_defeats.write(GrandChampionDefeated);
                    } else if stats.global_aristeia_reward > 0 {
                        global_aristeia.write(AddGlobalAristeia(stats.global_aristeia_reward));
                    }
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
        Option<&CachedPersistentState>,
        Option<&PlayerUsername>,
        Has<PersistenceReady>,
    )>,
    mut enemies: Query<(&GameRoom, &mut ProjectileBuffer, &EnemyIdentity), Without<PlayerId>>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
    mut transactions: MessageWriter<Transaction>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();
    let player_targets: Vec<(Entity, Vec2, GameRoom, bool)> = players
        .iter()
        .map(|(entity, position, room, health, _, _, _, _)| {
            (entity, position.0, *room, health.current > 0)
        })
        .collect();
    let mut pending_hits: Vec<(Entity, u32)> = Vec::new();

    for (room, mut buffer, identity) in &mut enemies {
        let damage = identity.stats(&settings).ranged_damage;
        let radius = identity.projectile_radius(&settings);
        if damage == 0 {
            buffer.simulate(room, tick, tick_secs, &settings, |_, _, _| None);
            continue;
        }
        buffer.simulate(room, tick, tick_secs, &settings, |source, pos, last_hit| {
            let is_enemy_shot = matches!(
                source,
                ProjectileSource::Enemy | ProjectileSource::GrandChampion
            );
            if !is_enemy_shot {
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

    let mut rng = rand::rng();
    for (player_entity, raw_damage) in pending_hits {
        let Ok((_, _, _, mut health, mut death_timer, cache, username, persistence_ready)) =
            players.get_mut(player_entity)
        else {
            continue;
        };
        if health.current == 0 {
            continue;
        }
        let armor = cache
            .map(|cache| {
                settings
                    .progression
                    .armor_at(cache.weapon(cache.equipped_weapon_id).armor_level)
            })
            .unwrap_or(0);
        apply_damage_to_player(&mut health, raw_damage, armor, &mut rng);
        if health.current == 0 {
            death_timer.remaining_seconds = settings.player.death_screen_duration_seconds;
            apply_death_penalty(
                &mut transactions,
                &settings,
                cache,
                username,
                persistence_ready,
                &mut rng,
            );
        }
    }
}
