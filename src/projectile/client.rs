use bevy::prelude::*;
use lightyear::prelude::Controlled;
use lightyear::{
    core::{tick::TickDuration, timeline::LocalTimeline},
    prediction::Predicted,
};
use rand::Rng;

use crate::{
    app::ClientState,
    combat::HitFlash,
    enemy::{EnemyHealth, EnemyIdentity, EnemyPosition},
    persistence::CachedPersistentState,
    player::PlayerId,
    player::protocol::{PlayerHealth, PlayerPosition},
    player::shared::apply_damage_to_player,
    projectile::protocol::{self, ProjectileBuffer, ProjectileSource},
    protocol::rooms::GameRoom,
    settings::GameSettings,
    shared::FixedGameplaySet,
};

#[cfg(feature = "gui")]
use crate::combat::{DamageHitId, DamagePopupKind, SpawnDamagePopup};

pub struct ProjectileClientPlugin;

impl Plugin for ProjectileClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                simulate_predicted_player_projectile_buffers,
                simulate_predicted_enemy_projectile_buffers,
            )
                .chain()
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(ClientState::Playing)),
        );
    }
}

fn simulate_predicted_player_projectile_buffers(
    mut players: Query<
        (
            Entity,
            &mut ProjectileBuffer,
            &GameRoom,
            Option<&CachedPersistentState>,
        ),
        (With<Predicted>, With<PlayerId>),
    >,
    mut enemies: Query<(
        Entity,
        &EnemyPosition,
        &EnemyIdentity,
        &GameRoom,
        &mut EnemyHealth,
        &mut HitFlash,
    )>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
    #[cfg(feature = "gui")] mut popups: MessageWriter<SpawnDamagePopup>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();
    let mut rng = rand::rng();

    for (owner, mut buffer, room, cache) in &mut players {
        let (damage_level, crit_chance) = match cache {
            Some(cache) => {
                let progress = cache.weapon(cache.equipped_weapon_id);
                (
                    progress.damage_level,
                    settings.progression.crit_chance_at(progress.crit_level),
                )
            }
            None => (0, 0.0),
        };
        #[cfg(feature = "gui")]
        let mut pending_popups = Vec::new();
        #[cfg(not(feature = "gui"))]
        let _ = owner;

        buffer.simulate(
            room,
            tick,
            tick_secs,
            &settings,
            |source, pos, hits, slot, generation| {
                let ProjectileSource::Weapon(weapon) = source else {
                    return None;
                };
                let stats = settings.weapons.get(weapon)?;
                let radius = stats.projectile_radius;
                let mut damage = settings.progression.damage_at(stats.damage, damage_level);
                let is_crit = rng.random::<f32>() < crit_chance;
                if is_crit {
                    damage = (damage as f32 * settings.progression.crit_damage_multiplier).round()
                        as u32;
                }

                enemies.iter_mut().find_map(
                    |(entity, enemy, identity, enemy_room, mut health, mut flash)| {
                        if enemy_room != room
                            || health.current == 0
                            || hits.contains(entity.to_bits())
                            || !protocol::overlaps(
                                pos,
                                enemy.0,
                                radius,
                                identity.kind.scale(settings.enemy.collision_radius),
                            )
                        {
                            return None;
                        }

                        health.current = health.current.saturating_sub(damage);
                        flash.trigger();

                        #[cfg(feature = "gui")]
                        if damage > 0 {
                            pending_popups.push(SpawnDamagePopup {
                                hit: DamageHitId {
                                    owner,
                                    slot,
                                    generation,
                                    target: entity,
                                    tick,
                                },
                                amount: damage,
                                position: enemy.0,
                                kind: DamagePopupKind::Outgoing,
                                is_crit,
                            });
                        }
                        #[cfg(not(feature = "gui"))]
                        let _ = (slot, generation, is_crit);

                        Some(entity.to_bits())
                    },
                )
            },
        );

        #[cfg(feature = "gui")]
        for popup in pending_popups {
            popups.write(popup);
        }
    }
}

fn simulate_predicted_enemy_projectile_buffers(
    mut enemies: Query<
        (Entity, &GameRoom, &mut ProjectileBuffer, &EnemyIdentity),
        (With<Predicted>, Without<PlayerId>),
    >,
    mut players: Query<
        (
            Entity,
            &PlayerPosition,
            &GameRoom,
            &mut PlayerHealth,
            &mut HitFlash,
            Option<&CachedPersistentState>,
        ),
        (With<Predicted>, With<Controlled>),
    >,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
    #[cfg(feature = "gui")] mut popups: MessageWriter<SpawnDamagePopup>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();
    let player_targets: Vec<(Entity, Vec2, GameRoom, bool)> = players
        .iter()
        .map(|(entity, position, room, health, _, _)| {
            (entity, position.0, *room, health.current > 0)
        })
        .collect();
    #[cfg(feature = "gui")]
    let mut pending_hits: Vec<(Entity, Entity, u16, u32, u32)> = Vec::new();
    #[cfg(not(feature = "gui"))]
    let mut pending_hits: Vec<(Entity, u32)> = Vec::new();

    for (owner, room, mut buffer, identity) in &mut enemies {
        #[cfg(not(feature = "gui"))]
        let _ = owner;
        let damage = identity.stats(&settings).ranged_damage;
        let radius = identity.projectile_radius(&settings);
        if damage == 0 {
            buffer.simulate(room, tick, tick_secs, &settings, |_, _, _, _, _| None);
            continue;
        }
        buffer.simulate(
            room,
            tick,
            tick_secs,
            &settings,
            |source, pos, hits, slot, generation| {
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
                        || hits.contains(player_entity.to_bits())
                        || !protocol::overlaps(
                            pos,
                            *player_position,
                            radius,
                            settings.player.collision_radius,
                        )
                    {
                        continue;
                    }
                    #[cfg(feature = "gui")]
                    pending_hits.push((*player_entity, owner, slot, generation, damage));
                    #[cfg(not(feature = "gui"))]
                    {
                        let _ = (slot, generation);
                        pending_hits.push((*player_entity, damage));
                    }
                    return Some(player_entity.to_bits());
                }
                None
            },
        );
    }

    let mut rng = rand::rng();
    #[cfg(feature = "gui")]
    for (player_entity, owner, slot, generation, raw_damage) in pending_hits {
        let Ok((_, position, _, mut health, mut flash, cache)) = players.get_mut(player_entity)
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
        let damage = apply_damage_to_player(&mut health, raw_damage, armor, &mut rng);
        if damage == 0 {
            continue;
        }
        flash.trigger();
        popups.write(SpawnDamagePopup {
            hit: DamageHitId {
                owner,
                slot,
                generation,
                target: player_entity,
                tick,
            },
            amount: damage,
            position: position.0,
            kind: DamagePopupKind::Incoming,
            is_crit: false,
        });
    }
    #[cfg(not(feature = "gui"))]
    for (player_entity, raw_damage) in pending_hits {
        let Ok((_, _, _, mut health, mut flash, cache)) = players.get_mut(player_entity) else {
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
        if apply_damage_to_player(&mut health, raw_damage, armor, &mut rng) > 0 {
            flash.trigger();
        }
    }
}
