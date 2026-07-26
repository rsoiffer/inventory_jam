use std::collections::HashSet;

use bevy::prelude::*;
use lightyear::{
    core::{
        tick::{Tick, TickDuration},
        timeline::LocalTimeline,
    },
    prediction::Predicted,
    prelude::PeerId,
};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;

use crate::{
    app::ServerState,
    enemy::protocol::{
        BossAttackState, BossPatternKind, EnemyAi, EnemyBehavior, EnemyHealth, EnemyIdentity,
        EnemyKind, EnemyPosition,
    },
    gate::{GateKind, GatePosition},
    player::protocol::PlayerHealth,
    player::{PlayerId, PlayerPosition},
    projectile::protocol::{ProjectileBuffer, ProjectileSource},
    protocol::rooms::{GameRoom, GameRooms},
    settings::GameSettings,
    weapon::protocol::WeaponCooldown,
};

pub(crate) fn simulate_enemy_ai(
    mut enemies: Query<(
        Has<Predicted>,
        &mut EnemyPosition,
        &mut EnemyAi,
        &EnemyIdentity,
    )>,
    players: Query<(&PlayerId, &PlayerPosition, &PlayerHealth, &GameRoom)>,
    gates: Query<(&GateKind, &GatePosition)>,
    app_state: Option<Res<State<ServerState>>>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let authoritative = app_state
        .as_ref()
        .is_some_and(|state| matches!(state.get(), ServerState::Hosting));
    let dt = tick_duration.0.as_secs_f32();
    let tick = local_timeline.tick().0;
    let arena = settings.world.arena_bounds();
    let gate_nearby_sq = (settings.gate.nearby_radius + settings.player.collision_radius).powi(2);
    let players_near_gate: HashSet<PeerId> = players
        .iter()
        .filter(|(_, _, health, room)| health.current > 0 && room.room == GameRooms::Arena)
        .filter(|(_, position, _, _)| {
            gates.iter().any(|(kind, gate)| {
                *kind == GateKind::ToSafezone
                    && position.0.distance_squared(gate.0) <= gate_nearby_sq
            })
        })
        .map(|(id, _, _, _)| id.0)
        .collect();
    let gate_aggro = settings.gate.nearby_detection_multiplier;

    for (predicted, mut position, mut ai, identity) in &mut enemies {
        if !(authoritative || predicted) {
            continue;
        }

        let mut rng = SmallRng::seed_from_u64(
            identity
                .ai_seed
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add(u64::from(tick)),
        );
        let stats = identity.stats(&settings);
        let arena = arena.inflate(-identity.kind.scale(settings.enemy.collision_radius));
        let (base_detection, base_leash, wander_radius) = match identity.kind {
            EnemyKind::Regular => (
                settings.enemy.detection_radius,
                settings.enemy.leash_radius,
                settings.enemy.wander_radius,
            ),
            EnemyKind::GrandChampion => (
                settings.global_aristeia.grand_champion_detection_radius,
                settings.global_aristeia.grand_champion_leash_radius,
                0.0,
            ),
        };
        let speed = stats.move_speed * dt;
        let aggro = |id: PeerId| {
            if players_near_gate.contains(&id) {
                gate_aggro
            } else {
                1.0
            }
        };

        let nearest = players
            .iter()
            .filter(|(_, _, health, room)| health.current > 0 && room.room == GameRooms::Arena)
            .map(|(id, pos, _, _)| {
                let detection = base_detection * aggro(id.0);
                (id.0, pos.0, position.0.distance_squared(pos.0), detection)
            })
            .filter(|(_, _, d, detection)| *d <= *detection * *detection)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(id, pos, d, _)| (id, pos, d));

        match ai.behavior {
            EnemyBehavior::Wandering => {
                if let Some((id, _, _)) = nearest {
                    ai.behavior = EnemyBehavior::Chasing(id);
                    ai.engage_retarget_seconds = 0.0;
                    continue;
                }
                if position.0.distance_squared(ai.wander_target) <= speed * speed {
                    let angle = rng.random_range(0.0..std::f32::consts::TAU);
                    let radius = rng.random_range(0.0..=wander_radius * wander_radius).sqrt();
                    ai.wander_target = (ai.home + Vec2::new(angle.cos(), angle.sin()) * radius)
                        .clamp(arena.min, arena.max);
                }
                move_toward(&mut position.0, ai.wander_target, speed);
            }
            EnemyBehavior::Chasing(target_id) => {
                let target = players
                    .iter()
                    .find(|(id, _, health, room)| {
                        id.0 == target_id && health.current > 0 && room.room == GameRooms::Arena
                    })
                    .map(|(_, p, _, _)| p.0);
                let leash = base_leash * aggro(target_id);
                if position.0.distance_squared(ai.home) > leash * leash || target.is_none() {
                    ai.behavior = EnemyBehavior::Returning;
                } else if let Some(target) = target {
                    ai.engage_retarget_seconds -= dt;
                    if identity.is_ranged {
                        if ai.engage_retarget_seconds <= 0.0
                            || position.0.distance_squared(ai.wander_target) <= speed * speed
                        {
                            let away = (position.0 - target).normalize_or_zero();
                            let tangent = Vec2::new(-away.y, away.x);
                            let side = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                            ai.wander_target = (target
                                + away * settings.enemy.standoff_distance
                                + tangent
                                    * side
                                    * rng.random_range(0.0..=settings.enemy.strafe_radius))
                            .clamp(arena.min, arena.max);
                            ai.engage_retarget_seconds = settings.enemy.engage_retarget_seconds;
                        }
                    } else if ai.engage_retarget_seconds <= 0.0 {
                        let r = settings.enemy.strafe_radius;
                        ai.wander_target = (target
                            + Vec2::new(rng.random_range(-r..r), rng.random_range(-r..r)))
                        .clamp(arena.min, arena.max);
                        ai.engage_retarget_seconds = settings.enemy.engage_retarget_seconds * 0.5;
                    }
                    move_toward(&mut position.0, ai.wander_target, speed);
                }
            }
            EnemyBehavior::Returning => {
                move_toward(&mut position.0, ai.home, speed);
                if position.0.distance_squared(ai.home) <= speed * speed {
                    position.0 = ai.home;
                    ai.wander_target = ai.home;
                    ai.behavior = EnemyBehavior::Wandering;
                }
            }
        }
        position.0 = position.0.clamp(arena.min, arena.max);
    }
}

pub(crate) fn fire_enemy_projectiles(
    mut enemies: Query<(
        Has<Predicted>,
        &EnemyPosition,
        &mut EnemyAi,
        &EnemyIdentity,
        &EnemyHealth,
        &mut ProjectileBuffer,
        &mut WeaponCooldown,
    )>,
    players: Query<(&PlayerId, &PlayerPosition, &PlayerHealth, &GameRoom)>,
    app_state: Option<Res<State<ServerState>>>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let authoritative = app_state
        .as_ref()
        .is_some_and(|state| matches!(state.get(), ServerState::Hosting));
    let dt = tick_duration.0.as_secs_f32();
    let tick = local_timeline.tick();

    for (predicted, position, mut ai, identity, health, mut buffer, mut cooldown) in &mut enemies {
        if !(authoritative || predicted) {
            continue;
        }
        let stats = identity.stats(&settings);
        if health.current == 0 || stats.ranged_damage == 0 {
            continue;
        }
        let EnemyBehavior::Chasing(target_id) = ai.behavior else {
            continue;
        };
        let Some(target) = players.iter().find_map(|(id, pos, player_health, room)| {
            (id.0 == target_id && player_health.current > 0 && room.room == GameRooms::Arena)
                .then_some(pos.0)
        }) else {
            continue;
        };

        if let Some(attack) = ai.boss.as_mut() {
            if identity.kind != EnemyKind::GrandChampion {
                continue;
            }
            let pattern_settings = match attack.pattern {
                BossPatternKind::Fan => &settings.global_aristeia.patterns.fan,
                BossPatternKind::Radial => &settings.global_aristeia.patterns.radial,
                BossPatternKind::Spiral => &settings.global_aristeia.patterns.spiral,
            };

            attack.pattern_elapsed_seconds += dt;
            if attack.pattern_elapsed_seconds >= pattern_settings.duration_seconds {
                attack.pattern = match attack.pattern {
                    BossPatternKind::Fan => BossPatternKind::Radial,
                    BossPatternKind::Radial => BossPatternKind::Spiral,
                    BossPatternKind::Spiral => BossPatternKind::Fan,
                };
                attack.pattern_elapsed_seconds = 0.0;
                attack.volley_accumulator = 0.0;
                continue;
            }

            attack.volley_accumulator += dt;
            while attack.volley_accumulator >= pattern_settings.volley_interval_seconds {
                attack.volley_accumulator -= pattern_settings.volley_interval_seconds;
                fire_boss_volley(
                    &mut buffer,
                    attack,
                    position.0,
                    target,
                    identity.kind.scale(settings.enemy.collision_radius),
                    &settings,
                    tick,
                );
            }
            continue;
        }

        if !identity.is_ranged || !cooldown.is_ready(tick) {
            continue;
        }

        let direction = (target - position.0).normalize_or_zero();
        if direction == Vec2::ZERO
            || position.0.distance_squared(target) > settings.enemy.projectile_range.powi(2)
        {
            continue;
        }

        buffer.insert(
            ProjectileSource::Enemy,
            position.0
                + direction
                    * (settings.enemy.collision_radius
                        + settings.enemy.projectile_radius
                        + settings.projectile.spawn_gap),
            direction * settings.enemy.projectile_speed,
            0,
            tick,
        );
        cooldown.restart(&local_timeline, &tick_duration, stats.attacks_per_second);
    }
}

fn fire_boss_volley(
    buffer: &mut ProjectileBuffer,
    attack: &mut BossAttackState,
    origin: Vec2,
    target: Vec2,
    collision_radius: f32,
    settings: &GameSettings,
    spawn_tick: Tick,
) {
    let speed = settings.global_aristeia.grand_champion_projectile_speed;
    let radius = settings.global_aristeia.grand_champion_projectile_radius;
    let spawn_gap = settings.projectile.spawn_gap;
    let patterns = &settings.global_aristeia.patterns;

    let directions: Vec<Vec2> = match attack.pattern {
        BossPatternKind::Fan => {
            let aim = (target - origin).normalize_or_zero();
            if aim == Vec2::ZERO {
                return;
            }
            let count = patterns.fan.projectile_count.max(1) as usize;
            let spread = patterns.fan.spread_degrees.to_radians();
            let start = -spread * 0.5;
            let step = if count == 1 {
                0.0
            } else {
                spread / (count - 1) as f32
            };
            (0..count)
                .map(|i| {
                    let radians = start + step * i as f32;
                    let (sin, cos) = radians.sin_cos();
                    Vec2::new(aim.x * cos - aim.y * sin, aim.x * sin + aim.y * cos)
                })
                .collect()
        }
        BossPatternKind::Radial => {
            let count = patterns.radial.projectile_count.max(1) as usize;
            let step = std::f32::consts::TAU / count as f32;
            let offset = if attack.radial_offset {
                step * 0.5
            } else {
                0.0
            };
            attack.radial_offset = !attack.radial_offset;
            (0..count)
                .map(|i| {
                    let angle = offset + step * i as f32;
                    Vec2::new(angle.cos(), angle.sin())
                })
                .collect()
        }
        BossPatternKind::Spiral => {
            let count = patterns.spiral.projectile_count.max(1) as usize;
            let step = std::f32::consts::TAU / count as f32;
            let base = attack.spiral_angle_radians;
            attack.spiral_angle_radians = (attack.spiral_angle_radians
                + patterns.spiral.rotation_degrees.to_radians())
            .rem_euclid(std::f32::consts::TAU);
            (0..count)
                .map(|i| {
                    let angle = base + step * i as f32;
                    Vec2::new(angle.cos(), angle.sin())
                })
                .collect()
        }
    };

    for direction in directions {
        if direction == Vec2::ZERO {
            continue;
        }
        buffer.insert(
            ProjectileSource::GrandChampion,
            origin + direction * (collision_radius + radius + spawn_gap),
            direction * speed,
            0,
            spawn_tick,
        );
    }
}

fn move_toward(position: &mut Vec2, target: Vec2, speed: f32) {
    let delta = target - *position;
    let distance = delta.length();
    if distance <= speed || distance == 0.0 {
        *position = target;
    } else {
        *position += delta / distance * speed;
    }
}
