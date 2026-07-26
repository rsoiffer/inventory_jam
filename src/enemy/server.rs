use crate::app::ServerState;
use crate::enemy::api::SpawnEnemy;
use crate::enemy::protocol::{
    EnemyHealth, EnemyKind, EnemyPosition, EnemySpawnerPosition, EnemyTierIndex,
};
use crate::gate::{GateKind, GatePosition};
use crate::player::protocol::PlayerHealth;
use crate::player::{PlayerId, PlayerPosition};
use crate::projectile::protocol::{ProjectileBuffer, ProjectileSource};
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::settings::{GameSettings, SpawnerTierSettings};
use crate::shared::FixedGameplaySet;
use crate::weapon::protocol::WeaponCooldown;
use bevy::prelude::*;
use lightyear::{
    core::{
        tick::{Tick, TickDuration},
        timeline::LocalTimeline,
    },
    prelude::*,
};
use rand::Rng;
use std::collections::HashSet;

pub struct EnemyServerPlugin;

impl Plugin for EnemyServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnEnemy>();
        app.configure_sets(
            FixedUpdate,
            (EnemySpawnSet::Request, EnemySpawnSet::Spawn).chain(),
        );
        app.add_systems(OnEnter(ServerState::Hosting), place_enemy_spawners);
        app.add_systems(
            FixedUpdate,
            (
                (despawn_distant_enemies, tick_enemy_spawners)
                    .chain()
                    .in_set(EnemySpawnSet::Request),
                spawn_requested_enemy.in_set(EnemySpawnSet::Spawn),
                (simulate_enemy_ai, fire_enemy_projectiles)
                    .chain()
                    .after(EnemySpawnSet::Spawn)
                    .before(FixedGameplaySet::Projectile),
            )
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EnemySpawnSet {
    Request,
    Spawn,
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct EnemyHome(pub Vec2);

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct EnemyWanderTarget(pub Vec2);

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct EnemySpawnerOwner(pub Entity);

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct SpawnerSpawnAccumulator(pub f32);

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct SpawnerTierIndex(pub u8);

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct EnemyStats {
    pub contact_damage: u32,
    pub ranged_damage: u32,
    pub attacks_per_second: f32,
    pub fragment_multiplier: f32,
    pub move_speed: f32,
    pub is_ranged: bool,
    pub personal_aristeia_reward: u32,
    pub global_aristeia_reward: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BossPatternKind {
    Fan,
    Radial,
    Spiral,
}

impl BossPatternKind {
    fn next(self) -> Self {
        match self {
            Self::Fan => Self::Radial,
            Self::Radial => Self::Spiral,
            Self::Spiral => Self::Fan,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct BossAttackState {
    pub pattern: BossPatternKind,
    pub pattern_elapsed_seconds: f32,
    pub volley_accumulator: f32,
    pub spiral_angle_radians: f32,
    pub radial_offset: bool,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct EnemyEngageRetarget(pub f32);

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) enum EnemyBehavior {
    #[default]
    Wandering,
    Chasing(PeerId),
    Returning,
}

fn place_enemy_spawners(
    mut commands: Commands,
    settings: Res<GameSettings>,
    existing: Query<(), With<EnemySpawnerPosition>>,
) {
    if !existing.is_empty() {
        return;
    }

    let bounds = settings
        .world
        .arena_bounds()
        .inflate(-settings.enemy.collision_radius);
    let mut positions = Vec::with_capacity(settings.spawner.count);
    for _ in 0..settings.spawner.count {
        let Some(position) = best_spawner_placement(&positions, bounds, &settings) else {
            break;
        };
        positions.push(position);
        let tier_index = settings.spawner.tier_index_for_distance(position.length());
        commands.spawn((
            EnemySpawnerPosition(position),
            SpawnerTierIndex(tier_index),
            SpawnerSpawnAccumulator::default(),
            GameRoom {
                room: GameRooms::Arena,
            },
            Replicate::to_clients(NetworkTarget::All),
            Name::new("Enemy Spawner"),
        ));
    }
}

fn best_spawner_placement(
    existing: &[Vec2],
    bounds: Rect,
    settings: &GameSettings,
) -> Option<Vec2> {
    let mut best = None;
    let mut best_clearance = -1.0;
    for _ in 0..settings.spawner.placement_candidate_count.max(1) {
        let candidate = Vec2::new(
            rand::random::<f32>() * (bounds.max.x - bounds.min.x) + bounds.min.x,
            rand::random::<f32>() * (bounds.max.y - bounds.min.y) + bounds.min.y,
        );
        let clearance = existing
            .iter()
            .map(|p| p.distance_squared(candidate))
            .fold(f32::MAX, f32::min);
        if clearance > best_clearance {
            best = Some(candidate);
            best_clearance = clearance;
        }
    }
    best
}

fn despawn_distant_enemies(
    mut commands: Commands,
    settings: Res<GameSettings>,
    players: Query<(&PlayerPosition, &PlayerHealth, &GameRoom)>,
    enemies: Query<(Entity, &EnemyPosition, &EnemyKind)>,
) {
    let active_players: Vec<Vec2> = players
        .iter()
        .filter(|(_, health, room)| health.current > 0 && room.room == GameRooms::Arena)
        .map(|(position, _, _)| position.0)
        .collect();

    if active_players.is_empty() {
        for (entity, _, kind) in &enemies {
            if *kind == EnemyKind::Regular {
                commands.entity(entity).despawn();
            }
        }
        return;
    }

    let retention_sq = settings.enemy.despawn_distance_from_players.powi(2);
    for (entity, position, kind) in &enemies {
        if *kind != EnemyKind::Regular {
            continue;
        }
        if active_players
            .iter()
            .all(|player| player.distance_squared(position.0) > retention_sq)
        {
            commands.entity(entity).despawn();
        }
    }
}

fn tick_enemy_spawners(
    time: Res<Time>,
    settings: Res<GameSettings>,
    players: Query<(&PlayerPosition, &PlayerHealth, &GameRoom)>,
    enemies: Query<(
        &EnemyPosition,
        &EnemyHealth,
        &EnemyKind,
        Option<&EnemySpawnerOwner>,
    )>,
    mut spawners: Query<(
        Entity,
        &EnemySpawnerPosition,
        &SpawnerTierIndex,
        &mut SpawnerSpawnAccumulator,
    )>,
    mut spawn_messages: MessageWriter<SpawnEnemy>,
) {
    let active_players: Vec<Vec2> = players
        .iter()
        .filter(|(_, health, room)| health.current > 0 && room.room == GameRooms::Arena)
        .map(|(position, _, _)| position.0)
        .collect();
    if active_players.is_empty() {
        return;
    }

    let activation_sq = settings.spawner.activation_radius.powi(2);
    let bounds = settings
        .world
        .arena_bounds()
        .inflate(-settings.enemy.collision_radius);
    let mut occupied: Vec<Vec2> = enemies
        .iter()
        .filter(|(_, health, kind, _)| health.current > 0 && **kind == EnemyKind::Regular)
        .map(|(position, _, _, _)| position.0)
        .collect();

    for (spawner_entity, spawner, tier_index, mut accumulator) in &mut spawners {
        let tier = spawner_tier(&settings, tier_index.0);
        let max_owned =
            ((settings.spawner.max_owned as f32) * tier.max_owned_multiplier).round() as usize;
        let spawn_rate = settings.spawner.spawn_rate_per_second * tier.spawn_rate_multiplier;

        let player_nearby = active_players
            .iter()
            .any(|player| player.distance_squared(spawner.0) <= activation_sq);
        if !player_nearby {
            accumulator.0 = 0.0;
            continue;
        }

        let owned = enemies
            .iter()
            .filter(|(_, health, kind, owner)| {
                health.current > 0
                    && **kind == EnemyKind::Regular
                    && owner.is_some_and(|owner| owner.0 == spawner_entity)
            })
            .count();
        if owned >= max_owned {
            accumulator.0 = 0.0;
            continue;
        }

        accumulator.0 += spawn_rate * time.delta_secs();
        let mut remaining_slots = max_owned - owned;
        while accumulator.0 >= 1.0 && remaining_slots > 0 {
            let Some(candidate) =
                best_spawn_candidate(spawner.0, &active_players, &occupied, bounds, &settings)
            else {
                break;
            };
            occupied.push(candidate);
            spawn_messages.write(SpawnEnemy::regular(candidate, spawner_entity, tier_index.0));
            accumulator.0 -= 1.0;
            remaining_slots -= 1;
        }
    }
}

fn best_spawn_candidate(
    spawner: Vec2,
    players: &[Vec2],
    occupied: &[Vec2],
    bounds: Rect,
    settings: &GameSettings,
) -> Option<Vec2> {
    let min_player_distance_sq = settings.spawner.spawn_min_distance_from_player.powi(2);
    let mut best = None;
    let mut best_clearance = -1.0;
    for _ in 0..settings.spawner.spawn_candidate_count.max(1) {
        let angle = rand::random::<f32>() * std::f32::consts::TAU;
        let radius = rand::random::<f32>().sqrt() * settings.spawner.spawn_radius;
        let candidate = spawner + Vec2::new(angle.cos(), angle.sin()) * radius;
        if !bounds.contains(candidate) {
            continue;
        }
        if players
            .iter()
            .any(|player| player.distance_squared(candidate) < min_player_distance_sq)
        {
            continue;
        }
        let clearance = occupied
            .iter()
            .map(|p| p.distance_squared(candidate))
            .fold(f32::MAX, f32::min);
        if clearance > best_clearance {
            best = Some(candidate);
            best_clearance = clearance;
        }
    }
    best
}

pub(crate) fn spawn_requested_enemy(
    mut commands: Commands,
    settings: Res<GameSettings>,
    spawners: Query<&EnemySpawnerPosition>,
    mut requests: MessageReader<SpawnEnemy>,
) {
    let mut rng = rand::rng();
    for request in requests.read() {
        let home = request
            .spawner
            .and_then(|spawner| spawners.get(spawner).ok())
            .map(|position| position.0)
            .unwrap_or(request.position);
        let stats = enemy_stats_for_request(&settings, request.kind, request.tier_index, &mut rng);
        let health = match request.kind {
            EnemyKind::Regular => {
                let tier = spawner_tier(
                    &settings,
                    request
                        .tier_index
                        .unwrap_or_else(|| settings.spawner.tier_index_for_distance(home.length())),
                );
                tier.health
            }
            EnemyKind::GrandChampion => settings.global_aristeia.grand_champion_health,
        };
        let tier_index = request.tier_index.unwrap_or(0);
        let buffer_capacity = match request.kind {
            EnemyKind::Regular => settings.projectile.buffer_capacity,
            EnemyKind::GrandChampion => {
                settings
                    .global_aristeia
                    .grand_champion_projectile_buffer_capacity
            }
        };
        spawn_enemy(
            &mut commands,
            request.position,
            home,
            health,
            request.kind,
            request.spawner,
            tier_index,
            stats,
            buffer_capacity,
        );
    }
}

fn enemy_stats_for_request(
    settings: &GameSettings,
    kind: EnemyKind,
    tier_index: Option<u8>,
    rng: &mut impl Rng,
) -> EnemyStats {
    match kind {
        EnemyKind::Regular => {
            let tier = spawner_tier(
                settings,
                tier_index.unwrap_or(settings.spawner.tiers.len().saturating_sub(1) as u8),
            );
            let is_ranged = rng.random::<f32>() < tier.ranged_chance;
            EnemyStats {
                contact_damage: if is_ranged { 0 } else { tier.contact_damage },
                ranged_damage: if is_ranged { tier.ranged_damage } else { 0 },
                attacks_per_second: settings.enemy.attacks_per_second,
                fragment_multiplier: tier.fragment_multiplier,
                move_speed: tier.move_speed,
                is_ranged,
                personal_aristeia_reward: tier.personal_aristeia_reward,
                global_aristeia_reward: tier.global_aristeia_reward,
            }
        }
        EnemyKind::GrandChampion => EnemyStats {
            contact_damage: settings.global_aristeia.grand_champion_contact_damage,
            ranged_damage: settings.global_aristeia.grand_champion_ranged_damage,
            attacks_per_second: 1.0,
            fragment_multiplier: settings.global_aristeia.grand_champion_fragment_multiplier,
            move_speed: settings.global_aristeia.grand_champion_move_speed,
            is_ranged: true,
            personal_aristeia_reward: settings
                .global_aristeia
                .grand_champion_personal_aristeia_reward,
            global_aristeia_reward: 0,
        },
    }
}

fn spawner_tier(settings: &GameSettings, tier_index: u8) -> &SpawnerTierSettings {
    match settings.spawner.tiers.get(tier_index as usize) {
        Some(tier) => tier,
        None => &settings.spawner.tiers[settings.spawner.tiers.len() - 1],
    }
}

fn spawn_enemy(
    commands: &mut Commands,
    position: Vec2,
    home: Vec2,
    max_health: u32,
    kind: EnemyKind,
    spawner: Option<Entity>,
    tier_index: u8,
    stats: EnemyStats,
    buffer_capacity: usize,
) -> Entity {
    let mut entity = commands.spawn((
        EnemyPosition(position),
        EnemyHome(home),
        EnemyWanderTarget(position),
        EnemyBehavior::Wandering,
        EnemyEngageRetarget::default(),
        EnemyHealth::new(max_health),
        EnemyStats { ..stats },
        EnemyTierIndex(tier_index),
        kind,
        ProjectileBuffer::new(buffer_capacity),
        WeaponCooldown::default(),
        GameRoom {
            room: GameRooms::Arena,
        },
        Replicate::to_clients(NetworkTarget::All),
        InterpolationTarget::to_clients(NetworkTarget::All),
        Name::new(match kind {
            EnemyKind::Regular => "Enemy",
            EnemyKind::GrandChampion => "Grand Champion",
        }),
    ));
    if let Some(spawner) = spawner {
        entity.insert(EnemySpawnerOwner(spawner));
    }
    if kind == EnemyKind::GrandChampion {
        entity.insert(BossAttackState {
            pattern: BossPatternKind::Fan,
            pattern_elapsed_seconds: 0.0,
            volley_accumulator: 0.0,
            spiral_angle_radians: 0.0,
            radial_offset: false,
        });
    }
    entity.id()
}

fn simulate_enemy_ai(
    time: Res<Time>,
    settings: Res<GameSettings>,
    mut enemies: Query<(
        &mut EnemyPosition,
        &EnemyHome,
        &mut EnemyWanderTarget,
        &mut EnemyBehavior,
        &mut EnemyEngageRetarget,
        &EnemyKind,
        &EnemyStats,
    )>,
    players: Query<(&PlayerId, &PlayerPosition, &PlayerHealth, &GameRoom)>,
    gates: Query<(&GateKind, &GatePosition)>,
) {
    let mut rng = rand::rng();
    let arena = settings.world.arena_bounds();
    let dt = time.delta_secs();
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
    let aggro_scale = |id: PeerId| {
        if players_near_gate.contains(&id) {
            gate_aggro
        } else {
            1.0
        }
    };

    for (mut position, home, mut wander_target, mut behavior, mut retarget, kind, stats) in
        &mut enemies
    {
        let collision_radius = kind.collision_radius(settings.enemy.collision_radius);
        let arena = arena.inflate(-collision_radius);
        let (base_detection, base_leash, wander_radius) = match kind {
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

        let nearest = players
            .iter()
            .filter(|(_, _, health, room)| health.current > 0 && room.room == GameRooms::Arena)
            .map(|(id, pos, _, _)| {
                let detection = base_detection * aggro_scale(id.0);
                (id.0, pos.0, position.0.distance_squared(pos.0), detection)
            })
            .filter(|(_, _, d, detection)| *d <= *detection * *detection)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(id, pos, d, _)| (id, pos, d));

        match *behavior {
            EnemyBehavior::Wandering => {
                if let Some((id, _, _)) = nearest {
                    *behavior = EnemyBehavior::Chasing(id);
                    retarget.0 = 0.0;
                    continue;
                }
                if position.0.distance_squared(wander_target.0) <= speed * speed {
                    let angle = rng.random_range(0.0..std::f32::consts::TAU);
                    let radius = rng.random_range(0.0..=wander_radius * wander_radius).sqrt();
                    wander_target.0 = (home.0 + Vec2::new(angle.cos(), angle.sin()) * radius)
                        .clamp(arena.min, arena.max);
                }
                move_toward(&mut position.0, wander_target.0, speed);
            }
            EnemyBehavior::Chasing(target_id) => {
                let target = players
                    .iter()
                    .find(|(id, _, health, room)| {
                        id.0 == target_id && health.current > 0 && room.room == GameRooms::Arena
                    })
                    .map(|(_, p, _, _)| p.0);
                let leash = base_leash * aggro_scale(target_id);
                if position.0.distance_squared(home.0) > leash * leash || target.is_none() {
                    *behavior = EnemyBehavior::Returning;
                } else if let Some(target) = target {
                    retarget.0 -= dt;
                    if stats.is_ranged {
                        if retarget.0 <= 0.0
                            || position.0.distance_squared(wander_target.0) <= speed * speed
                        {
                            wander_target.0 =
                                ranged_engage_point(position.0, target, &settings, &mut rng, arena);
                            retarget.0 = settings.enemy.engage_retarget_seconds;
                        }
                        move_toward(&mut position.0, wander_target.0, speed);
                    } else if retarget.0 <= 0.0 {
                        let jitter = Vec2::new(
                            rng.random_range(
                                -settings.enemy.strafe_radius..settings.enemy.strafe_radius,
                            ),
                            rng.random_range(
                                -settings.enemy.strafe_radius..settings.enemy.strafe_radius,
                            ),
                        );
                        wander_target.0 = (target + jitter).clamp(arena.min, arena.max);
                        retarget.0 = settings.enemy.engage_retarget_seconds * 0.5;
                        move_toward(&mut position.0, wander_target.0, speed);
                    } else {
                        move_toward(&mut position.0, wander_target.0, speed);
                    }
                }
            }
            EnemyBehavior::Returning => {
                move_toward(&mut position.0, home.0, speed);
                if position.0.distance_squared(home.0) <= speed * speed {
                    position.0 = home.0;
                    wander_target.0 = home.0;
                    *behavior = EnemyBehavior::Wandering;
                }
            }
        }
        position.0 = position.0.clamp(arena.min, arena.max);
    }
}

fn ranged_engage_point(
    enemy: Vec2,
    player: Vec2,
    settings: &GameSettings,
    rng: &mut impl Rng,
    arena: Rect,
) -> Vec2 {
    let away = (enemy - player).normalize_or_zero();
    let tangent = Vec2::new(-away.y, away.x);
    let side = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
    let standoff = away * settings.enemy.standoff_distance
        + tangent * side * rng.random_range(0.0..=settings.enemy.strafe_radius);
    (player + standoff).clamp(arena.min, arena.max)
}

fn fire_enemy_projectiles(
    time: Res<Time>,
    settings: Res<GameSettings>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    mut regular_enemies: Query<
        (
            &EnemyPosition,
            &EnemyBehavior,
            &EnemyStats,
            &EnemyHealth,
            &mut ProjectileBuffer,
            &mut WeaponCooldown,
        ),
        (With<EnemyKind>, Without<BossAttackState>),
    >,
    mut bosses: Query<(
        &EnemyPosition,
        &EnemyBehavior,
        &EnemyStats,
        &EnemyHealth,
        &EnemyKind,
        &mut ProjectileBuffer,
        &mut BossAttackState,
    )>,
    players: Query<(&PlayerId, &PlayerPosition, &PlayerHealth, &GameRoom)>,
) {
    for (position, behavior, stats, health, mut buffer, mut cooldown) in &mut regular_enemies {
        if !stats.is_ranged || health.current == 0 || stats.ranged_damage == 0 {
            continue;
        }
        let EnemyBehavior::Chasing(target_id) = *behavior else {
            continue;
        };
        if !cooldown.is_ready(local_timeline.tick()) {
            continue;
        }
        let Some(target) = players.iter().find_map(|(id, pos, player_health, room)| {
            (id.0 == target_id && player_health.current > 0 && room.room == GameRooms::Arena)
                .then_some(pos.0)
        }) else {
            continue;
        };

        let direction = (target - position.0).normalize_or_zero();
        if direction == Vec2::ZERO {
            continue;
        }
        let range_sq = settings.enemy.projectile_range.powi(2);
        if position.0.distance_squared(target) > range_sq {
            continue;
        }

        let spawn_position = position.0
            + direction
                * (settings.enemy.collision_radius
                    + settings.enemy.projectile_radius
                    + settings.projectile.spawn_gap);
        buffer.insert(
            ProjectileSource::Enemy,
            spawn_position,
            direction * settings.enemy.projectile_speed,
            0,
            local_timeline.tick(),
        );
        cooldown.restart(&local_timeline, &tick_duration, stats.attacks_per_second);
    }

    let dt = time.delta_secs();
    for (position, behavior, stats, health, kind, mut buffer, mut attack) in &mut bosses {
        if *kind != EnemyKind::GrandChampion || health.current == 0 || stats.ranged_damage == 0 {
            continue;
        }
        let EnemyBehavior::Chasing(target_id) = *behavior else {
            continue;
        };
        let Some(target) = players.iter().find_map(|(id, pos, player_health, room)| {
            (id.0 == target_id && player_health.current > 0 && room.room == GameRooms::Arena)
                .then_some(pos.0)
        }) else {
            continue;
        };

        let pattern_settings = match attack.pattern {
            BossPatternKind::Fan => &settings.global_aristeia.patterns.fan,
            BossPatternKind::Radial => &settings.global_aristeia.patterns.radial,
            BossPatternKind::Spiral => &settings.global_aristeia.patterns.spiral,
        };

        attack.pattern_elapsed_seconds += dt;
        if attack.pattern_elapsed_seconds >= pattern_settings.duration_seconds {
            attack.pattern = attack.pattern.next();
            attack.pattern_elapsed_seconds = 0.0;
            attack.volley_accumulator = 0.0;
            continue;
        }

        attack.volley_accumulator += dt;
        while attack.volley_accumulator >= pattern_settings.volley_interval_seconds {
            attack.volley_accumulator -= pattern_settings.volley_interval_seconds;
            fire_boss_volley(
                &mut buffer,
                &mut attack,
                position.0,
                target,
                kind.collision_radius(settings.enemy.collision_radius),
                &settings,
                local_timeline.tick(),
            );
        }
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
                .map(|i| rotate_vec2(aim, start + step * i as f32))
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
        let spawn_position = origin + direction * (collision_radius + radius + spawn_gap);
        buffer.insert(
            ProjectileSource::GrandChampion,
            spawn_position,
            direction * speed,
            0,
            spawn_tick,
        );
    }
}

fn rotate_vec2(direction: Vec2, radians: f32) -> Vec2 {
    let (sin, cos) = radians.sin_cos();
    Vec2::new(
        direction.x * cos - direction.y * sin,
        direction.x * sin + direction.y * cos,
    )
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
