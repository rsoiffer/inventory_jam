use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::weapon::protocol::WeaponId;

pub struct WeaponStationProtocolPlugin;

impl Plugin for WeaponStationProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<WeaponStationPosition>()
            .replicate()
            .add_linear_interpolation();
        app.component::<WeaponStationId>().replicate();
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Deref)]
pub struct WeaponStationId(pub WeaponId);

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct WeaponStationPosition(pub Vec2);

impl Ease for WeaponStationPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            WeaponStationPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}
