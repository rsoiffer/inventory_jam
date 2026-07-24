use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    app::ServerState,
    enemy::{EnemyKind, api::SpawnEnemy, server::EnemySpawnSet},
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
                (apply_global_aristeia, drain_global_aristeia).chain(),
                spawn_grand_champion_when_ready.in_set(EnemySpawnSet::Request),
            )
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
