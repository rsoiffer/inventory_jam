use bevy::prelude::{
    App, Component, Curve, Deref, DerefMut, Ease, FunctionCurve, Interval, Plugin, Vec2,
};
use lightyear::prelude::{
    AppChannelExt, AppComponentExt, AppMessageExt, ChannelMode, ChannelSettings, NetworkDirection,
    PeerId, ReliableSettings, Tick,
};
use serde::{Deserialize, Serialize};

pub struct ProjectileProtocolPlugin;

impl Plugin for ProjectileProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<PlayerProjectile>().replicate();
        app.component::<ProjectilePosition>()
            .replicate()
            // .predict()
            // .add_linear_interpolation()
            ;

        // Reliable: this is a single, low-frequency, user-triggered event
        // rather than continuous state, so we want it to always eventually
        // arrive instead of silently vanishing under simulated latency/loss.
        app.add_channel::<DebugFireChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..Default::default()
        })
        .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<DebugFireProjectileAt>()
            .add_direction(NetworkDirection::ClientToServer);
    }
}

/// Client -> server request to fire a debug projectile at a world position.
///
/// The server does not trust `origin` for anything but this debug tool.
/// `request_id` is chosen by the client and echoed back verbatim so both
/// sides can build the exact same `PreSpawned` hash from it (see
/// `client::debug_fire_hash` / `server::debug_fire_hash`) — the default
/// tick-based `PreSpawned` hash assumes both sides spawn on the "same" tick
/// and that no two projectiles share a tick, neither of which holds for a
/// client-triggered request that crosses the network.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct DebugFireProjectileAt {
    pub origin: Vec2,
    pub request_id: u64,
}

pub struct DebugFireChannel;

/// Authoritative attack snapshot carried by a projectile.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct PlayerProjectile {
    pub owner: PeerId,
    pub origin: Vec2,
    pub direction: Vec2,
    pub speed_per_tick: f32,
    pub damage: u32,
    pub max_range: f32,
    pub radius: f32,
    pub expire_time: ProjectileLifetime,
}

/// If a projectile is "smart" (e.g. homing projectile), we want to
/// synchronize the projectile position to avoid desync.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Deref, DerefMut)]
pub struct ProjectilePosition(pub Vec2);

impl Ease for ProjectilePosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            ProjectilePosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectileLifetime {
    expire_time: Tick,
}

impl ProjectileLifetime {
    pub fn new(current_tick: &Tick, max_range: f32, speed_per_tick: f32) -> Self {
        let travel_ticks = (max_range / speed_per_tick.max(0.001)).ceil() as u32;

        Self {
            expire_time: *current_tick + Tick(travel_ticks.saturating_add(2).min(u16::MAX as u32)),
        }
    }

    pub fn is_expired(&self, current_tick: &Tick) -> bool {
        current_tick > &self.expire_time
    }
}
