use bevy::prelude::*;
use lightyear::core::timeline::LocalTimeline;
use lightyear::prelude::{MessageReceiver, PeerId, PreSpawned, RemoteId, Tick, server::ClientOf};

use crate::app::AppState;
use crate::projectile::protocol::{DebugFireProjectileAt, PlayerProjectile, ProjectileLifetime};
use crate::projectile::shared::{SpawnProjectile, simulate_server_projectiles};
use crate::shared::FixedGameplaySet;

const DEBUG_FIRE_SPEED_PER_TICK: f32 = 4.0;
const DEBUG_FIRE_MAX_RANGE: f32 = 1000.0;
const DEBUG_FIRE_RADIUS: f32 = 8.0;

pub struct ProjectileServerPlugin;

impl Plugin for ProjectileServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            // Automatic periodic debug fire disabled for now.
            (
                handle_debug_fire_at_requests_server,
                simulate_server_projectiles,
            )
                .chain()
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(AppState::Hosting)),
        );
    }
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

/// Hash used to match a client-prespawned debug-fire projectile with the
/// server's authoritative spawn for the same request.
///
/// The default `PreSpawned` hash is derived from the spawn tick plus the set
/// of replicated component *types* on the entity — it ignores field values,
/// so it collides whenever two projectiles of the same shape spawn on the
/// same tick (which the debug-fire loop routinely does, both here and on the
/// client). It also doesn't work across the network: the server only spawns
/// in reaction to a message that has already taken some (variable) amount of
/// network time to arrive, so its own current tick at that point routinely
/// differs from the client's tick at send time. Hashing the client-chosen
/// `request_id` (salted with the owner, so two players' shots — or two shots
/// from the same player in the same tick — can't collide) instead gives both
/// sides a value that's identical regardless of latency or how many
/// projectiles spawn in a tick.
fn debug_fire_hash(owner: PeerId, request_id: u64) -> u64 {
    use core::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    owner.hash(&mut hasher);
    request_id.hash(&mut hasher);
    hasher.finish()
}

/// Server-side handler for manual debug fire requests: spawns one
/// authoritative projectile per request, owned by the requesting client.
fn handle_debug_fire_at_requests_server(
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
