use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub const ENEMY_SIZE: f32 = 50.0;

pub struct EnemyProtocolPlugin;

impl Plugin for EnemyProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<EnemyPosition>()
            .replicate()
            .add_linear_interpolation();

        app.component::<EnemyHealth>().replicate();
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct EnemyPosition(pub Vec2);

impl Ease for EnemyPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            EnemyPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyHealth {
    pub current: u32,
    pub maximum: u32,
}

impl EnemyHealth {
    pub const fn new(maximum: u32) -> Self {
        Self {
            current: maximum,
            maximum,
        }
    }
}
