use bevy::prelude::*;
use bevy::time::Real;
use lightyear::prelude::*;

use crate::app::ServerState;
use crate::debug_stats::ServerDebugStats;
use crate::enemy::EnemyIdentity;
use crate::player::PlayerId;

const TICK_HZ_SAMPLE_SECONDS: f32 = 0.5;

pub struct DebugStatsServerPlugin;

impl Plugin for DebugStatsServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimTickCounter>();
        app.add_systems(OnEnter(ServerState::Hosting), spawn_server_debug_stats);
        app.add_systems(
            FixedUpdate,
            count_sim_tick.run_if(in_state(ServerState::Hosting)),
        );
        app.add_systems(
            Update,
            update_server_debug_stats.run_if(in_state(ServerState::Hosting)),
        );
    }
}

#[derive(Resource, Default)]
struct SimTickCounter {
    ticks: u32,
    elapsed: f32,
}

fn spawn_server_debug_stats(mut commands: Commands, existing: Query<(), With<ServerDebugStats>>) {
    if !existing.is_empty() {
        return;
    }
    commands.spawn((
        ServerDebugStats::default(),
        Replicate::to_clients(NetworkTarget::All),
        Name::new("Server Debug Stats"),
    ));
}

fn count_sim_tick(mut counter: ResMut<SimTickCounter>) {
    counter.ticks = counter.ticks.saturating_add(1);
}

fn update_server_debug_stats(
    time: Res<Time<Real>>,
    mut counter: ResMut<SimTickCounter>,
    players: Query<(), With<PlayerId>>,
    enemies: Query<(), With<EnemyIdentity>>,
    mut stats: Query<&mut ServerDebugStats>,
) {
    let Ok(mut stats) = stats.single_mut() else {
        return;
    };

    stats.players = players.iter().len() as u32;
    stats.enemies = enemies.iter().len() as u32;

    counter.elapsed += time.delta_secs();
    if counter.elapsed < TICK_HZ_SAMPLE_SECONDS {
        return;
    }

    stats.tick_hz = counter.ticks as f32 / counter.elapsed;
    counter.ticks = 0;
    counter.elapsed = 0.0;
}
