use crate::weapon::protocol::WeaponKind;
use bevy::prelude::{
    App, Component, Curve, Deref, DerefMut, Ease, FunctionCurve, Interval, Plugin, Vec2,
};
use lightyear::prelude::{
    AppComponentExt, InterpolationRegistrationExt, PeerId, PredictionBuilderExt, Tick,
};
use serde::{Deserialize, Serialize};

pub struct ProjectileProtocolPlugin;

impl Plugin for ProjectileProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<PlayerProjectile>().replicate();
        app.component::<ProjectileImpact>().replicate();
        app.component::<ProjectilePosition>()
            .replicate()
            .predict()
            .add_linear_interpolation();
    }
}

/// Authoritative attack snapshot carried by a projectile.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct PlayerProjectile {
    pub owner: PeerId,
    pub weapon: WeaponKind,
    pub origin: Vec2,
    pub direction: Vec2,
    pub speed_per_tick: f32,
    pub damage: u32,
    pub max_range: f32,
    pub radius: f32,
    pub expire_time: ProjectileLifetime,
}

/// Short-lived replicated impact event represented as an entity.
///
/// Clients use this to create local hit VFX. The server has already resolved
/// the hit and applied damage before this component is replicated.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct ProjectileImpact {
    pub position: Vec2,
    pub weapon: WeaponKind,
    pub damage: u32,
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
