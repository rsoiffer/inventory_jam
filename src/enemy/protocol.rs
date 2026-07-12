use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct EnemyPosition(pub Vec2);

impl Ease for EnemyPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            EnemyPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

pub fn register(app: &mut App) {
    app.component::<EnemyPosition>()
        .replicate()
        .add_linear_interpolation();
}
