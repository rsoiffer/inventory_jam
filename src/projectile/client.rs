use bevy::prelude::*;
use lightyear::prelude::Controlled;
use lightyear::{
    core::{tick::TickDuration, timeline::LocalTimeline},
    prediction::Predicted,
};

use crate::{
    app::ClientState,
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
    mut players: Query<(&mut ProjectileBuffer, &GameRoom), (With<Predicted>, With<PlayerId>)>,
    enemies: Query<(
        Entity,
        &EnemyPosition,
        &EnemyIdentity,
        &GameRoom,
        &EnemyHealth,
    )>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();

    for (mut buffer, room) in &mut players {
        buffer.simulate(room, tick, tick_secs, &settings, |source, pos, last_hit| {
            let ProjectileSource::Weapon(weapon) = source else {
                return None;
            };
            let radius = settings
                .weapons
                .get(weapon)
                .map(|stats| stats.projectile_radius)?;
            enemies
                .iter()
                .find_map(|(entity, enemy, identity, enemy_room, health)| {
                    (enemy_room == room
                        && health.current > 0
                        && entity.to_bits() != last_hit
                        && protocol::overlaps(
                            pos,
                            enemy.0,
                            radius,
                            identity.kind.scale(settings.enemy.collision_radius),
                        ))
                    .then_some(entity.to_bits())
                })
        });
    }
}

fn simulate_predicted_enemy_projectile_buffers(
    mut enemies: Query<
        (&GameRoom, &mut ProjectileBuffer, &EnemyIdentity),
        (With<Predicted>, Without<PlayerId>),
    >,
    mut players: Query<
        (
            Entity,
            &PlayerPosition,
            &GameRoom,
            &mut PlayerHealth,
            Option<&CachedPersistentState>,
        ),
        (With<Predicted>, With<Controlled>),
    >,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();
    let player_targets: Vec<(Entity, Vec2, GameRoom, bool)> = players
        .iter()
        .map(|(entity, position, room, health, _)| (entity, position.0, *room, health.current > 0))
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
        let Ok((_, _, _, mut health, cache)) = players.get_mut(player_entity) else {
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
    }
}
