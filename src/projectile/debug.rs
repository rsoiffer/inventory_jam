use bevy::prelude::*;
use lightyear::core::{tick::TickDuration, timeline::LocalTimeline};
use lightyear::prelude::{PeerId, PreSpawned, Tick};

#[cfg(feature = "server")]
use lightyear::prelude::{MessageReceiver, RemoteId, server::ClientOf};

#[cfg(all(feature = "client", feature = "gui"))]
use lightyear::prelude::{Client, MessageSender};

#[cfg(feature = "client")]
use crate::app::LocalUsername;
use crate::{
    app::client_id_from_username,
    projectile::protocol::{PlayerProjectile, ProjectileLifetime},
    projectile::shared::SpawnProjectile,
};

#[cfg(any(feature = "server", all(feature = "client", feature = "gui")))]
use crate::projectile::protocol::DebugFireProjectileAt;

#[cfg(all(feature = "client", feature = "gui"))]
use crate::projectile::protocol::DebugFireChannel;

const DEBUG_FIRE_PERIOD_SECS: f32 = 2.0;
const DEBUG_FIRE_ORIGIN_X: f32 = -500.0;
const DEBUG_FIRE_SPEED_PER_TICK: f32 = 4.0;
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

fn build_projectile_at(owner: PeerId, origin: Vec2, current_tick: &Tick) -> PlayerProjectile {
    PlayerProjectile {
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
    }
}

fn build_projectile(owner: PeerId, spawn_y: f32, current_tick: &Tick) -> (PlayerProjectile, Vec2) {
    let origin = Vec2::new(DEBUG_FIRE_ORIGIN_X, spawn_y);
    (build_projectile_at(owner, origin, current_tick), origin)
}

/// Hash used to match a client-prespawned debug-fire projectile with the
/// server's authoritative spawn for the same request.
///
/// The default `PreSpawned` hash is derived from whichever tick is current
/// on the *local* machine when the entity is inserted. That works for the
/// periodic debug fire because both sides independently decide to fire on
/// the same deterministic schedule without any message crossing the
/// network. It does not work here: the server only spawns in reaction to a
/// message that has already taken some (variable) amount of network time to
/// arrive, so its own current tick at that point routinely differs from the
/// client's tick at send time. Hashing the client-chosen `request_id`
/// (salted with the owner, so two players' shots can't collide) instead
/// gives both sides a value that's identical regardless of latency.
#[cfg(any(feature = "server", all(feature = "client", feature = "gui")))]
fn debug_fire_hash(owner: PeerId, request_id: u64) -> u64 {
    use core::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    owner.hash(&mut hasher);
    request_id.hash(&mut hasher);
    hasher.finish()
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
        prespawn: PreSpawned::default(),
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
            prespawn: PreSpawned::default(),
        });
    }
}

/// Cursor-fire requests captured in `Update` and drained in `FixedUpdate`.
///
/// `ButtonInput::just_pressed` is only true for the single frame the press
/// happened. Reading it directly in `FixedUpdate` drops presses whenever a
/// rendered frame runs zero fixed ticks (routinely the case here: 64Hz fixed
/// tick vs. an AutoVsync'd display), so we capture it every frame in
/// `Update` instead, where a press can never be missed.
#[cfg(all(feature = "client", feature = "gui"))]
#[derive(Resource, Default)]
pub(crate) struct PendingDebugFireRequests(Vec<Vec2>);

#[cfg(all(feature = "client", feature = "gui"))]
pub(crate) fn queue_debug_fire_at_cursor_client(
    keys: Res<ButtonInput<KeyCode>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window>,
    mut pending: ResMut<PendingDebugFireRequests>,
) {
    if !keys.pressed(KeyCode::KeyA) {
        return;
    }

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let (camera, camera_transform) = *camera;
    let Ok(origin) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return;
    };

    pending.0.push(origin);
}

/// Manual debug fire: pressing A pre-spawns a projectile under the mouse
/// cursor and asks the server to spawn the authoritative copy at the same
/// position, so the two reconcile via `PreSpawned` like any other shot.
#[cfg(all(feature = "client", feature = "gui"))]
pub(crate) fn fire_debug_projectile_at_cursor_client(
    mut commands: Commands,
    mut pending: ResMut<PendingDebugFireRequests>,
    local_timeline: Res<LocalTimeline>,
    username: Res<LocalUsername>,
    mut sender: Single<&mut MessageSender<DebugFireProjectileAt>, With<Client>>,
    mut next_request_id: Local<u64>,
) {
    if pending.0.is_empty() {
        return;
    }

    let current_tick = local_timeline.tick();
    let owner = PeerId::Netcode(client_id_from_username(&username.0));

    for origin in pending.0.drain(..) {
        let projectile = build_projectile_at(owner, origin, &current_tick);
        let request_id = *next_request_id;
        *next_request_id += 1;
        let hash = debug_fire_hash(owner, request_id);

        commands.queue(SpawnProjectile {
            projectile,
            spawn_position: origin,
            is_authoritative: false,
            prespawn: PreSpawned::new(hash),
        });

        info!(
            ?origin,
            ?owner,
            request_id,
            "sending debug fire request to server"
        );
        sender.send::<DebugFireChannel>(DebugFireProjectileAt { origin, request_id });
    }
}

/// Server-side handler for manual debug fire requests: spawns one
/// authoritative projectile per request, owned by the requesting client.
#[cfg(feature = "server")]
pub(crate) fn handle_debug_fire_at_requests_server(
    mut commands: Commands,
    mut requests: Query<(&RemoteId, &mut MessageReceiver<DebugFireProjectileAt>), With<ClientOf>>,
    local_timeline: Res<LocalTimeline>,
) {
    let current_tick = local_timeline.tick();

    for (remote_id, mut receiver) in &mut requests {
        for request in receiver.receive() {
            info!(
                owner = ?remote_id.0,
                origin = ?request.origin,
                request_id = request.request_id,
                "received debug fire request from client"
            );
            let projectile = build_projectile_at(remote_id.0, request.origin, &current_tick);
            let hash = debug_fire_hash(remote_id.0, request.request_id);

            commands.queue(SpawnProjectile {
                projectile,
                spawn_position: request.origin,
                is_authoritative: true,
                prespawn: PreSpawned::new(hash),
            });
        }
    }
}
