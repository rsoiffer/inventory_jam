use bevy::prelude::*;
use lightyear::core::{tick::TickDuration, timeline::LocalTimeline};

use crate::{
    app::ServerState,
    combat::HitFlash,
    enemy::{EnemyHealth, EnemyIdentity, EnemyKind, EnemyPosition},
    fragment::api::SpawnFragmentPool,
    persistence::{CachedPersistentState, PersistenceReady, Transaction},
    player::PlayerId,
    player::PlayerPosition,
    player::PlayerUsername,
    player::api::AddAristeiaPoints,
    player::protocol::{PlayerAristeia, PlayerHealth},
    player::server::{PlayerDeathTimer, apply_death_penalty},
    player::shared::{apply_damage_to_player, armor_from_cache},
    projectile::protocol::{ProjectileBuffer, ProjectileSource},
    projectile::shared::roll_weapon_damage,
    projectile::spatial::build_combat_spatial_hash,
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
        &mut HitFlash,
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
    let targets = build_combat_spatial_hash(
        enemies
            .iter()
            .filter(|(_, _, _, health, _, _)| health.current > 0)
            .map(|(entity, position, room, _, identity, _)| {
                (
                    entity,
                    position.0,
                    *room,
                    identity.kind.scale(settings.enemy.collision_radius),
                )
            }),
    );

    for (owner_id, room, mut buffer, aristeia, cache) in &mut players {
        let owner = owner_id.0;
        let personal_aristeia = aristeia.current;
        let progress = cache.weapon(cache.equipped_weapon_id);
        let damage_level = progress.damage_level;
        let crit_chance = settings.progression.crit_chance_at(progress.crit_level);
        buffer.simulate(
            room,
            tick,
            tick_secs,
            &settings,
            &targets,
            |source, pos, hit_id, _, generation, pierce_remaining| {
                let ProjectileSource::Weapon(weapon) = source else {
                    return;
                };
                let Some(weapon_stats) = settings.weapons.get(weapon) else {
                    return;
                };
                let (damage, _) = roll_weapon_damage(
                    &settings,
                    weapon_stats.damage,
                    damage_level,
                    crit_chance,
                    generation,
                    pierce_remaining,
                );
                let Ok((entity, _, _, mut health, identity, mut flash)) =
                    enemies.get_mut(Entity::from_bits(hit_id))
                else {
                    return;
                };
                if health.current == 0 {
                    return;
                }
                health.current = health.current.saturating_sub(damage);
                flash.trigger();
                if health.current > 0 {
                    return;
                }
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
            },
        );
    }
}

fn simulate_enemy_projectile_buffers(
    mut players: Query<(
        Entity,
        &PlayerPosition,
        &GameRoom,
        &mut PlayerHealth,
        &mut HitFlash,
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
    let player_radius = settings.player.collision_radius;
    let targets = build_combat_spatial_hash(
        players
            .iter()
            .filter(|(_, _, _, health, _, _, _, _, _)| health.current > 0)
            .map(|(entity, position, room, _, _, _, _, _, _)| {
                (entity, position.0, *room, player_radius)
            }),
    );
    let mut pending_hits: Vec<(Entity, u32)> = Vec::new();

    for (room, mut buffer, identity) in &mut enemies {
        let damage = identity.stats(&settings).ranged_damage;
        if damage == 0 {
            continue;
        }
        buffer.simulate(
            room,
            tick,
            tick_secs,
            &settings,
            &targets,
            |_, _, hit_id, _, _, _| {
                pending_hits.push((Entity::from_bits(hit_id), damage));
            },
        );
    }

    let mut rng = rand::rng();
    for (player_entity, raw_damage) in pending_hits {
        let Ok((
            _,
            _,
            _,
            mut health,
            mut flash,
            mut death_timer,
            cache,
            username,
            persistence_ready,
        )) = players.get_mut(player_entity)
        else {
            continue;
        };
        if health.current == 0 {
            continue;
        }
        if apply_damage_to_player(
            &mut health,
            raw_damage,
            armor_from_cache(&settings, cache),
            &mut rng,
        ) > 0
        {
            flash.trigger();
        }
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
