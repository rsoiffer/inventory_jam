use crate::app::AppState;
use crate::projectile::shared::simulate_server_projectiles;
use crate::shared::FixedGameplaySet;
use bevy::prelude::*;

#[cfg(feature = "gui")]
use crate::app::{LocalUsername, client_id_from_username};
#[cfg(feature = "gui")]
use crate::projectile::protocol::{
    DebugFireChannel, DebugFireProjectileAt, PlayerProjectile, ProjectileLifetime,
};
#[cfg(feature = "gui")]
use crate::projectile::shared::SpawnProjectile;
#[cfg(feature = "gui")]
use lightyear::core::timeline::LocalTimeline;
#[cfg(feature = "gui")]
use lightyear::prelude::{Client, MessageSender, PeerId, PreSpawned, Tick};

pub struct ProjectileClientPlugin;

impl Plugin for ProjectileClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            // Automatic periodic debug fire disabled for now.
            (simulate_server_projectiles)
                .chain()
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(AppState::Playing)),
        );

        #[cfg(feature = "gui")]
        {
            app.init_resource::<PendingDebugFireRequests>();
            app.add_systems(
                Update,
                queue_debug_fire_at_cursor_client.run_if(in_state(AppState::Playing)),
            );
            app.add_systems(
                FixedUpdate,
                fire_debug_projectile_at_cursor_client
                    .in_set(FixedGameplaySet::Projectile)
                    .run_if(in_state(AppState::Playing)),
            );
        }
    }
}

#[cfg(feature = "gui")]
const DEBUG_FIRE_SPEED_PER_TICK: f32 = 4.0;
#[cfg(feature = "gui")]
const DEBUG_FIRE_MAX_RANGE: f32 = 1000.0;
#[cfg(feature = "gui")]
const DEBUG_FIRE_RADIUS: f32 = 8.0;

#[cfg(feature = "gui")]
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
/// server's authoritative spawn for the same request. See
/// `server::debug_fire_hash` for the server-side counterpart and the
/// rationale for hashing `(owner, request_id)` instead of the default
/// tick-based hash.
#[cfg(feature = "gui")]
fn debug_fire_hash(owner: PeerId, request_id: u64) -> u64 {
    use core::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    owner.hash(&mut hasher);
    request_id.hash(&mut hasher);
    hasher.finish()
}

/// Cursor-fire requests captured in `Update` and drained in `FixedUpdate`.
///
/// `ButtonInput::just_pressed` is only true for the single frame the press
/// happened. Reading it directly in `FixedUpdate` drops presses whenever a
/// rendered frame runs zero fixed ticks (routinely the case here: 64Hz fixed
/// tick vs. an AutoVsync'd display), so we capture it every frame in
/// `Update` instead, where a press can never be missed.
#[cfg(feature = "gui")]
#[derive(Resource, Default)]
struct PendingDebugFireRequests(Vec<Vec2>);

#[cfg(feature = "gui")]
fn queue_debug_fire_at_cursor_client(
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
#[cfg(feature = "gui")]
fn fire_debug_projectile_at_cursor_client(
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
