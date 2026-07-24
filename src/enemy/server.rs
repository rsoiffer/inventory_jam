use crate::app::ServerState;
use crate::enemy::api::SpawnEnemy;
use crate::enemy::protocol::{EnemyHealth, EnemyKind, EnemyPosition, EnemySpawnerPosition};
use crate::player::protocol::PlayerHealth;
use crate::player::{PlayerId, PlayerPosition};
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::settings::GameSettings;
use bevy::prelude::*;
use lightyear::prelude::*;
use rand::Rng;

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
                simulate_enemy_ai.after(EnemySpawnSet::Spawn),
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
        commands.spawn((
            EnemySpawnerPosition(position),
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
    mut spawners: Query<(Entity, &EnemySpawnerPosition, &mut SpawnerSpawnAccumulator)>,
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

    for (spawner_entity, spawner, mut accumulator) in &mut spawners {
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
        if owned >= settings.spawner.max_owned {
            accumulator.0 = 0.0;
            continue;
        }

        accumulator.0 += settings.spawner.spawn_rate_per_second * time.delta_secs();
        let mut remaining_slots = settings.spawner.max_owned - owned;
        while accumulator.0 >= 1.0 && remaining_slots > 0 {
            let Some(candidate) =
                best_spawn_candidate(spawner.0, &active_players, &occupied, bounds, &settings)
            else {
                break;
            };
            occupied.push(candidate);
            spawn_messages.write(SpawnEnemy::regular(candidate, spawner_entity));
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
    for request in requests.read() {
        let health = match request.kind {
            EnemyKind::Regular => settings.enemy.max_health,
            EnemyKind::GrandChampion => settings.global_aristeia.grand_champion_health,
        };
        let home = request
            .spawner
            .and_then(|spawner| spawners.get(spawner).ok())
            .map(|position| position.0)
            .unwrap_or(request.position);
        spawn_enemy(
            &mut commands,
            request.position,
            home,
            health,
            request.kind,
            request.spawner,
        );
    }
}

fn spawn_enemy(
    commands: &mut Commands,
    position: Vec2,
    home: Vec2,
    max_health: u32,
    kind: EnemyKind,
    spawner: Option<Entity>,
) -> Entity {
    let mut entity = commands.spawn((
        EnemyPosition(position),
        EnemyHome(home),
        EnemyWanderTarget(position),
        EnemyBehavior::Wandering,
        EnemyHealth::new(max_health),
        kind,
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
        &EnemyKind,
    )>,
    players: Query<(&PlayerId, &PlayerPosition, &PlayerHealth, &GameRoom)>,
) {
    let mut rng = rand::rng();
    let arena = settings.world.arena_bounds();
    let dt = time.delta_secs();

    for (mut position, home, mut wander_target, mut behavior, kind) in &mut enemies {
        let collision_radius = kind.collision_radius(settings.enemy.collision_radius);
        let arena = arena.inflate(-collision_radius);
        let (speed, detection, leash, wander_radius) = match kind {
            EnemyKind::Regular => (
                settings.enemy.move_speed * dt,
                settings.enemy.detection_radius,
                settings.enemy.leash_radius,
                settings.enemy.wander_radius,
            ),
            EnemyKind::GrandChampion => (
                settings.global_aristeia.grand_champion_move_speed * dt,
                settings.global_aristeia.grand_champion_detection_radius,
                settings.global_aristeia.grand_champion_leash_radius,
                0.0,
            ),
        };

        let nearest = players
            .iter()
            .filter(|(_, _, health, room)| health.current > 0 && room.room == GameRooms::Arena)
            .map(|(id, pos, _, _)| (id.0, pos.0, position.0.distance_squared(pos.0)))
            .filter(|(_, _, d)| *d <= detection * detection)
            .min_by(|a, b| a.2.total_cmp(&b.2));

        match *behavior {
            EnemyBehavior::Wandering => {
                if let Some((id, _, _)) = nearest {
                    *behavior = EnemyBehavior::Chasing(id);
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
                if position.0.distance_squared(home.0) > leash * leash || target.is_none() {
                    *behavior = EnemyBehavior::Returning;
                } else if let Some(target) = target {
                    move_toward(&mut position.0, target, speed);
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

fn move_toward(position: &mut Vec2, target: Vec2, speed: f32) {
    let delta = target - *position;
    let distance = delta.length();
    if distance <= speed || distance == 0.0 {
        *position = target;
    } else {
        *position += delta / distance * speed;
    }
}
