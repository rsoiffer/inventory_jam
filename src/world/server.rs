use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    app::ServerState,
    enemy::{EnemyHealth, EnemyKind, EnemyPosition, api::SpawnEnemy, server::EnemySpawnSet},
    player::protocol::{PlayerHealth, PlayerPosition},
    protocol::rooms::{GameRoom, GameRooms},
    settings::GameSettings,
    world::{GlobalAristeia, api::AddGlobalAristeia},
};

pub struct WorldServerPlugin;

impl Plugin for WorldServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AddGlobalAristeia>();
        app.add_systems(OnEnter(ServerState::Hosting), spawn_global_aristeia);
        app.add_systems(
            FixedUpdate,
            (
                run_enemy_population_manager.in_set(EnemySpawnSet::Request),
                apply_global_aristeia,
                drain_global_aristeia,
                spawn_grand_champion_when_ready,
            )
                .chain()
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

fn spawn_global_aristeia(
    mut commands: Commands,
    settings: Res<GameSettings>,
    existing: Query<(), With<GlobalAristeia>>,
) {
    if !existing.is_empty() {
        return;
    }
    commands.spawn((
        GlobalAristeia::new(settings.global_aristeia.threshold),
        Replicate::to_clients(NetworkTarget::All),
        Name::new("Global Aristeia"),
    ));
}

fn run_enemy_population_manager(
    mut commands: Commands,
    settings: Res<GameSettings>,
    players: Query<(&PlayerPosition, &PlayerHealth, &GameRoom)>,
    enemies: Query<(Entity, &EnemyPosition, &EnemyHealth, &EnemyKind, &GameRoom)>,
    mut spawn_messages: MessageWriter<SpawnEnemy>,
) {
    let active_players: Vec<Vec2> = players
        .iter()
        .filter(|(_, health, room)| health.current > 0 && room.room == GameRooms::Arena)
        .map(|(position, _, _)| position.0)
        .collect();

    if active_players.is_empty() {
        for (entity, _, _, kind, _) in &enemies {
            if *kind == EnemyKind::Regular {
                commands.entity(entity).despawn();
            }
        }
        return;
    }

    let retention_sq = settings.enemy.despawn_distance_from_players.powi(2);
    for (entity, position, health, kind, room) in &enemies {
        if *kind != EnemyKind::Regular || health.current == 0 || room.room != GameRooms::Arena {
            continue;
        }
        if active_players
            .iter()
            .all(|player| player.distance_squared(position.0) > retention_sq)
        {
            commands.entity(entity).despawn();
        }
    }

    let regular_positions: Vec<Vec2> = enemies
        .iter()
        .filter(|(_, _, health, kind, room)| {
            health.current > 0 && **kind == EnemyKind::Regular && room.room == GameRooms::Arena
        })
        .map(|(_, position, _, _, _)| position.0)
        .collect();

    let desired_total =
        (active_players.len() * settings.enemy.target_per_player).min(settings.enemy.limit);
    let mut remaining = desired_total
        .saturating_sub(regular_positions.len())
        .min(settings.enemy.max_spawns_per_tick);
    if remaining == 0 {
        return;
    }

    let bounds = settings
        .world
        .arena_bounds()
        .inflate(-settings.enemy.collision_radius);
    let mut occupied = regular_positions;
    'players: for player in active_players
        .iter()
        .cycle()
        .take(active_players.len() * settings.enemy.max_spawns_per_tick.max(1))
    {
        if remaining == 0 {
            break;
        }
        if let Some(candidate) = best_spawn_candidate(*player, &occupied, bounds, &settings) {
            occupied.push(candidate);
            spawn_messages.write(SpawnEnemy::regular(candidate));
            remaining -= 1;
        }
        if remaining == 0 {
            break 'players;
        }
    }
}

fn best_spawn_candidate(
    center: Vec2,
    occupied: &[Vec2],
    bounds: Rect,
    settings: &GameSettings,
) -> Option<Vec2> {
    let mut best = None;
    let mut best_clearance = -1.0;
    for _ in 0..settings.enemy.spawn_candidate_count.max(1) {
        let angle = rand::random::<f32>() * std::f32::consts::TAU;
        let min = settings.enemy.spawn_min_distance_from_player;
        let max = settings.enemy.spawn_radius.max(min);
        let radius = (min * min + rand::random::<f32>() * (max * max - min * min)).sqrt();
        let candidate = center + Vec2::new(angle.cos(), angle.sin()) * radius;
        if !bounds.contains(candidate) {
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

fn apply_global_aristeia(
    mut requests: MessageReader<AddGlobalAristeia>,
    mut global: Query<&mut GlobalAristeia>,
) {
    let Ok(mut global) = global.single_mut() else {
        return;
    };
    for AddGlobalAristeia(amount) in requests.read() {
        global.current = global.current.saturating_add(*amount).min(global.maximum);
    }
}

fn drain_global_aristeia(
    settings: Res<GameSettings>,
    mut counter: Local<u16>,
    mut global: Query<&mut GlobalAristeia>,
) {
    *counter = counter.saturating_add(1);
    if *counter < settings.global_aristeia.drain_interval_ticks {
        return;
    }
    *counter = 0;
    if let Ok(mut global) = global.single_mut() {
        global.current = global
            .current
            .saturating_sub(settings.global_aristeia.drain_amount);
    }
}

fn spawn_grand_champion_when_ready(
    mut global: Query<&mut GlobalAristeia>,
    enemy_kinds: Query<&EnemyKind>,
    mut spawns: MessageWriter<SpawnEnemy>,
) {
    let Ok(mut global) = global.single_mut() else {
        return;
    };
    let champion_exists = enemy_kinds
        .iter()
        .any(|kind| *kind == EnemyKind::GrandChampion);
    if global.current < global.maximum || champion_exists {
        return;
    }
    global.current = 0;
    spawns.write(SpawnEnemy::grand_champion(Vec2::ZERO));
    info!("Global Aristeia filled: spawning the Grand Champion");
}
