use bevy::prelude::*;
use lightyear::core::{tick::TickDuration, timeline::LocalTimeline};
use lightyear::prelude::{PeerId, Tick};

#[cfg(feature = "server")]
use lightyear::prelude::{RemoteId, server::ClientOf};

#[cfg(feature = "client")]
use crate::app::LocalUsername;
use crate::{
    app::client_id_from_username,
    projectile::protocol::{PlayerProjectile, ProjectileLifetime},
    projectile::shared::SpawnProjectile,
};

const DEBUG_FIRE_PERIOD_SECS: f32 = 2.0;
const DEBUG_FIRE_ORIGIN_X: f32 = -500.0;
const DEBUG_FIRE_SPEED_PER_TICK: f32 = 14.0;
const DEBUG_FIRE_MAX_RANGE: f32 = 1000.0;
const DEBUG_FIRE_RADIUS: f32 = 8.0;

/// Hardcoded test-player-name -> spawn Y mapping.
///
/// Matches the `--user` values this branch is launched with, so each
/// connected test client gets its own fixed, easily distinguishable
/// projectile lane.
const DEBUG_PLAYERS: &[(&str, f32)] = &[("Player1", 100.0), ("Player2", -100.0)];

/// Returns the current tick if it's time to fire, based on
/// `DEBUG_FIRE_PERIOD_SECS`.
fn debug_fire_tick(local_timeline: &LocalTimeline, tick_duration: &TickDuration) -> Option<Tick> {
    let ticks_per_period = (DEBUG_FIRE_PERIOD_SECS / tick_duration.0.as_secs_f32())
        .round()
        .max(1.0) as u32;
    let current_tick = local_timeline.tick();

    current_tick
        .0
        .is_multiple_of(ticks_per_period)
        .then_some(current_tick)
}

fn build_projectile(owner: PeerId, spawn_y: f32, current_tick: &Tick) -> (PlayerProjectile, Vec2) {
    let origin = Vec2::new(DEBUG_FIRE_ORIGIN_X, spawn_y);
    let projectile = PlayerProjectile {
        owner,
        origin,
        direction: Vec2::X,
        speed_per_tick: DEBUG_FIRE_SPEED_PER_TICK,
        damage: 0,
        max_range: DEBUG_FIRE_MAX_RANGE,
        radius: DEBUG_FIRE_RADIUS,
        expire_time: ProjectileLifetime::new(
            current_tick,
            DEBUG_FIRE_MAX_RANGE,
            DEBUG_FIRE_SPEED_PER_TICK,
        ),
    };
    (projectile, origin)
}

/// Client-side prediction of this client's own debug projectile.
///
/// Only predicts the local player's own shot (looked up by `LocalUsername`
/// against `DEBUG_PLAYERS`); the server independently fires the
/// authoritative version for every connected test player, so this
/// reconciles via `PreSpawned` exactly like a real weapon fire would.
#[cfg(feature = "client")]
pub(crate) fn fire_debug_projectile_client(
    mut commands: Commands,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    username: Res<LocalUsername>,
) {
    let Some(current_tick) = debug_fire_tick(&local_timeline, &tick_duration) else {
        return;
    };

    let Some(&(_, spawn_y)) = DEBUG_PLAYERS.iter().find(|(name, _)| *name == username.0) else {
        return;
    };

    let owner = PeerId::Netcode(client_id_from_username(&username.0));
    let (projectile, origin) = build_projectile(owner, spawn_y, &current_tick);

    commands.queue(SpawnProjectile {
        projectile,
        spawn_position: origin,
        is_authoritative: false,
    });
}

/// Server-authoritative debug fire: one projectile per connected test
/// player, matched against `DEBUG_PLAYERS` by comparing each connection's
/// `RemoteId` to the id derived from the hardcoded player name.
#[cfg(feature = "server")]
pub(crate) fn fire_debug_projectiles_server(
    mut commands: Commands,
    clients: Query<&RemoteId, With<ClientOf>>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
) {
    let Some(current_tick) = debug_fire_tick(&local_timeline, &tick_duration) else {
        return;
    };

    for remote_id in &clients {
        let owner = remote_id.0;

        let Some(&(_, spawn_y)) = DEBUG_PLAYERS
            .iter()
            .find(|(name, _)| PeerId::Netcode(client_id_from_username(name)) == owner)
        else {
            continue;
        };

        let (projectile, origin) = build_projectile(owner, spawn_y, &current_tick);

        commands.queue(SpawnProjectile {
            projectile,
            spawn_position: origin,
            is_authoritative: true,
        });
    }
}
