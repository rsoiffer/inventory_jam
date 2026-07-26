use crate::app::ServerState;
use crate::combat::HitFlash;
use crate::enemy::api::SpawnEnemy;
use crate::enemy::protocol::{
    BossAttackState, BossPatternKind, EnemyAi, EnemyHealth, EnemyIdentity, EnemyKind,
    EnemyPosition, EnemySpawnerPosition,
};
use crate::enemy::shared::{fire_enemy_projectiles, simulate_enemy_ai};
use crate::player::PlayerPosition;
use crate::player::protocol::PlayerHealth;
use crate::projectile::protocol::ProjectileBuffer;
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::settings::GameSettings;
use crate::shared::FixedGameplaySet;
use crate::weapon::protocol::WeaponCooldown;
use bevy::prelude::*;
use lightyear::prelude::*;
use rand::Rng;

pub struct EnemyServerPlugin;

impl Plugin for EnemyServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnEnemy>();
        app.configure_sets(
            FixedUpdate,
            (EnemySpawnSet::Request, EnemySpawnSet::Spawn)
                .chain()
                .before(FixedGameplaySet::Enemy),
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
                    .in_set(FixedGameplaySet::Enemy),
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
pub(crate) struct EnemySpawnerOwner(pub Entity);

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct SpawnerSpawnAccumulator(pub f32);

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct SpawnerTierIndex(pub u8);

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
    enemies: Query<(Entity, &EnemyPosition, &EnemyIdentity)>,
) {
    let active_players: Vec<Vec2> = players
        .iter()
        .filter(|(_, health, room)| health.current > 0 && room.room == GameRooms::Arena)
        .map(|(position, _, _)| position.0)
        .collect();

    if active_players.is_empty() {
        for (entity, _, identity) in &enemies {
            if identity.kind == EnemyKind::Regular {
                commands.entity(entity).despawn();
            }
        }
        return;
    }

    let retention_sq = settings.enemy.despawn_distance_from_players.powi(2);
    for (entity, position, identity) in &enemies {
        if identity.kind != EnemyKind::Regular {
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
        &EnemyIdentity,
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
        .filter(|(_, health, identity, _)| {
            health.current > 0 && identity.kind == EnemyKind::Regular
        })
        .map(|(position, _, _, _)| position.0)
        .collect();

    for (spawner_entity, spawner, tier_index, mut accumulator) in &mut spawners {
        let tier = settings.spawner.tier(tier_index.0);
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
            .filter(|(_, health, identity, owner)| {
                health.current > 0
                    && identity.kind == EnemyKind::Regular
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
        let ai_seed = rng.random::<u64>();
        let identity = match request.kind {
            EnemyKind::Regular => {
                let tier_index = request
                    .tier_index
                    .unwrap_or_else(|| settings.spawner.tier_index_for_distance(home.length()));
                EnemyIdentity {
                    kind: EnemyKind::Regular,
                    tier_index,
                    is_ranged: rng.random::<f32>()
                        < settings.spawner.tier(tier_index).ranged_chance,
                    ai_seed,
                }
            }
            EnemyKind::GrandChampion => EnemyIdentity {
                kind: EnemyKind::GrandChampion,
                tier_index: 0,
                is_ranged: true,
                ai_seed,
            },
        };
        let kind = identity.kind;
        let max_health = identity.max_health(&settings);
        let buffer_capacity = identity.projectile_buffer_capacity(&settings);
        let mut entity = commands.spawn((
            EnemyPosition(request.position),
            EnemyAi {
                home,
                wander_target: home,
                behavior: Default::default(),
                engage_retarget_seconds: 0.0,
                boss: (kind == EnemyKind::GrandChampion).then_some(BossAttackState {
                    pattern: BossPatternKind::Fan,
                    pattern_elapsed_seconds: 0.0,
                    volley_accumulator: 0.0,
                    spiral_angle_radians: 0.0,
                    radial_offset: false,
                }),
            },
            EnemyHealth {
                current: max_health,
                maximum: max_health,
            },
            HitFlash::default(),
            identity,
            ProjectileBuffer::new(buffer_capacity),
            WeaponCooldown::default(),
            GameRoom {
                room: GameRooms::Arena,
            },
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::All),
            Name::new(match kind {
                EnemyKind::Regular => "Enemy",
                EnemyKind::GrandChampion => "Grand Champion",
            }),
        ));
        if let Some(spawner) = request.spawner {
            entity.insert(EnemySpawnerOwner(spawner));
        }
    }
}
