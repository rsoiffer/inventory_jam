use crate::app::ServerState;
use crate::enemy::api::SpawnEnemy;
use crate::enemy::protocol::{EnemyHealth, EnemyKind, EnemyPosition};
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
        app.add_systems(
            FixedUpdate,
            (
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

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) enum EnemyBehavior {
    #[default]
    Wandering,
    Chasing(PeerId),
    Returning,
}

pub(crate) fn spawn_requested_enemy(
    mut commands: Commands,
    settings: Res<GameSettings>,
    mut requests: MessageReader<SpawnEnemy>,
) {
    for request in requests.read() {
        let health = match request.kind {
            EnemyKind::Regular => settings.enemy.max_health,
            EnemyKind::GrandChampion => settings.global_aristeia.grand_champion_health,
        };
        spawn_enemy(&mut commands, request.position, health, request.kind);
    }
}

fn spawn_enemy(
    commands: &mut Commands,
    position: Vec2,
    max_health: u32,
    kind: EnemyKind,
) -> Entity {
    commands
        .spawn((
            EnemyPosition(position),
            EnemyHome(position),
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
        ))
        .id()
}

fn simulate_enemy_ai(
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
    let arena = settings
        .world
        .arena_bounds()
        .inflate(-settings.enemy.collision_radius);

    for (mut position, home, mut wander_target, mut behavior, kind) in &mut enemies {
        let (speed, detection, leash, wander_radius) = match kind {
            EnemyKind::Regular => (
                settings.enemy.move_speed_per_tick,
                settings.enemy.detection_radius,
                settings.enemy.leash_radius,
                settings.enemy.wander_radius,
            ),
            EnemyKind::GrandChampion => (
                settings.global_aristeia.grand_champion_move_speed_per_tick,
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
