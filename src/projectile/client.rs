use bevy::prelude::*;
use lightyear::prelude::Controlled;
use lightyear::{
    core::{tick::TickDuration, timeline::LocalTimeline},
    prediction::Predicted,
};

use crate::{
    app::ClientState,
    combat::HitFlash,
    enemy::{EnemyHealth, EnemyIdentity, EnemyPosition},
    persistence::CachedPersistentState,
    player::PlayerId,
    player::protocol::{PlayerHealth, PlayerPosition},
    player::shared::{apply_damage_to_player, armor_from_cache},
    projectile::protocol::{ProjectileBuffer, ProjectileSource},
    projectile::shared::roll_weapon_damage,
    projectile::spatial::build_combat_spatial_hash,
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
    let targets = build_combat_spatial_hash(
        enemies
            .iter()
            .filter(|(_, _, _, _, health, _)| health.current > 0)
            .map(|(entity, position, identity, room, _, _)| {
                (
                    entity,
                    position.0,
                    *room,
                    identity.kind.scale(settings.enemy.collision_radius),
                )
            }),
    );

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
            &targets,
            |source, _pos, hit_id, slot, generation, pierce_remaining| {
                let ProjectileSource::Weapon(weapon) = source else {
                    return;
                };
                let Some(stats) = settings.weapons.get(weapon) else {
                    return;
                };
                let (damage, is_crit) = roll_weapon_damage(
                    &settings,
                    stats.damage,
                    damage_level,
                    crit_chance,
                    generation,
                    pierce_remaining,
                );
                let Ok((entity, enemy, _, _, mut health, mut flash)) =
                    enemies.get_mut(Entity::from_bits(hit_id))
                else {
                    return;
                };
                if health.current == 0 {
                    return;
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
                        },
                        amount: damage,
                        position: enemy.0,
                        kind: DamagePopupKind::Outgoing,
                        is_crit,
                    });
                }
                #[cfg(not(feature = "gui"))]
                let _ = (slot, generation, is_crit, entity, enemy);
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
    let player_radius = settings.player.collision_radius;
    let targets = build_combat_spatial_hash(
        players
            .iter()
            .filter(|(_, _, _, health, _, _)| health.current > 0)
            .map(|(entity, position, room, _, _, _)| (entity, position.0, *room, player_radius)),
    );
    let mut pending_hits: Vec<(Entity, Entity, u16, u32, u32)> = Vec::new();

    for (owner, room, mut buffer, identity) in &mut enemies {
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
            |_, _, hit_id, slot, generation, _| {
                pending_hits.push((Entity::from_bits(hit_id), owner, slot, generation, damage));
            },
        );
    }

    let mut rng = rand::rng();
    for (player_entity, owner, slot, generation, raw_damage) in pending_hits {
        let Ok((_, position, _, mut health, mut flash, cache)) = players.get_mut(player_entity)
        else {
            continue;
        };
        if health.current == 0 {
            continue;
        }
        let damage = apply_damage_to_player(
            &mut health,
            raw_damage,
            armor_from_cache(&settings, cache),
            &mut rng,
        );
        if damage == 0 {
            continue;
        }
        flash.trigger();
        #[cfg(feature = "gui")]
        popups.write(SpawnDamagePopup {
            hit: DamageHitId {
                owner,
                slot,
                generation,
                target: player_entity,
            },
            amount: damage,
            position: position.0,
            kind: DamagePopupKind::Incoming,
            is_crit: false,
        });
        #[cfg(not(feature = "gui"))]
        let _ = (owner, slot, generation, position);
    }
}
