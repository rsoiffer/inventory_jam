use bevy::prelude::*;
#[cfg(feature = "dev")]
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

#[cfg(feature = "dev")]
use crate::protocol::inputs::PlayerAction;
use crate::{
    app::ServerState,
    enemy::{EnemyIdentity, EnemyKind, api::SpawnEnemy, server::EnemySpawnSet},
    player::PlayerId,
    player::protocol::{PlayerAristeia, PlayerHealth, PlayerPosition},
    protocol::rooms::{GameRoom, GameRooms},
    settings::GameSettings,
    shared::FixedGameplaySet,
    world::{
        GlobalAristeia,
        api::{AddGlobalAristeia, GrandChampionDefeated},
    },
};

#[cfg(feature = "dev")]
const DEBUG_GLOBAL_ARISTEIA_BOOST: u32 = 500;

pub struct WorldServerPlugin;

impl Plugin for WorldServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AddGlobalAristeia>();
        app.add_message::<GrandChampionDefeated>();
        app.add_systems(OnEnter(ServerState::Hosting), spawn_global_aristeia);
        app.add_systems(
            FixedUpdate,
            (
                #[cfg(feature = "dev")]
                debug_boost_global_aristeia,
                (apply_global_aristeia, drain_global_aristeia).chain(),
                spawn_grand_champion_when_ready.in_set(EnemySpawnSet::Request),
                reset_global_aristeia_on_boss_death.after(FixedGameplaySet::Projectile),
            )
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

#[cfg(feature = "dev")]
fn debug_boost_global_aristeia(
    players: Query<&ActionState<PlayerAction>, With<PlayerId>>,
    mut global_aristeia: MessageWriter<AddGlobalAristeia>,
) {
    for actions in &players {
        if actions.just_pressed(&PlayerAction::DebugBoostGlobalAristeia) {
            global_aristeia.write(AddGlobalAristeia(DEBUG_GLOBAL_ARISTEIA_BOOST));
            info!("Dev: boosted global Aristeia by {DEBUG_GLOBAL_ARISTEIA_BOOST}");
        }
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

fn apply_global_aristeia(
    mut requests: MessageReader<AddGlobalAristeia>,
    mut global: Query<&mut GlobalAristeia>,
) {
    let Ok(mut global) = global.single_mut() else {
        return;
    };
    if global.boss_active {
        for _ in requests.read() {}
        return;
    }
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
        if global.boss_active {
            return;
        }
        global.current = global
            .current
            .saturating_sub(settings.global_aristeia.drain_amount);
    }
}

fn spawn_grand_champion_when_ready(
    settings: Res<GameSettings>,
    mut global: Query<&mut GlobalAristeia>,
    enemy_identities: Query<&EnemyIdentity>,
    players: Query<(&PlayerPosition, &PlayerAristeia, &PlayerHealth, &GameRoom)>,
    mut spawns: MessageWriter<SpawnEnemy>,
) {
    let Ok(mut global) = global.single_mut() else {
        return;
    };
    let champion_exists = enemy_identities
        .iter()
        .any(|identity| identity.kind == EnemyKind::GrandChampion);
    if champion_exists {
        global.boss_active = true;
        return;
    }
    if global.boss_active || global.current < global.maximum {
        return;
    }

    let Some(anchor) = players
        .iter()
        .filter(|(_, _, health, room)| health.current > 0 && room.room == GameRooms::Arena)
        .max_by(|a, b| {
            a.1.current
                .cmp(&b.1.current)
                .then_with(|| a.1.progress.cmp(&b.1.progress))
        })
        .map(|(position, _, _, _)| position.0)
    else {
        return;
    };

    let arena = settings
        .world
        .arena_bounds()
        .inflate(-settings.enemy.collision_radius * 4.0);
    let angle = rand::random::<f32>() * std::f32::consts::TAU;
    let offset = Vec2::new(angle.cos(), angle.sin())
        * settings.global_aristeia.grand_champion_spawn_distance;
    let spawn_position = (anchor + offset).clamp(arena.min, arena.max);

    global.boss_active = true;
    spawns.write(SpawnEnemy::grand_champion(spawn_position));
    info!(
        ?spawn_position,
        "Global Aristeia filled: spawning the Grand Champion"
    );
}

fn reset_global_aristeia_on_boss_death(
    mut defeats: MessageReader<GrandChampionDefeated>,
    mut global: Query<&mut GlobalAristeia>,
) {
    if defeats.read().next().is_none() {
        return;
    }
    if let Ok(mut global) = global.single_mut() {
        global.current = 0;
        global.boss_active = false;
        info!("Grand Champion defeated: global Aristeia reset");
    }
}
