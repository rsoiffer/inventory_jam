use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub struct EnemyProtocolPlugin;

impl Plugin for EnemyProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<EnemyPosition>()
            .replicate()
            .add_linear_interpolation();

        app.component::<EnemyHealth>().replicate();
        app.component::<EnemyKind>().replicate();
        app.component::<EnemySpawnerPosition>().replicate();
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

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Regular,
    GrandChampion,
}

impl EnemyKind {
    pub fn collision_radius(self, base: f32) -> f32 {
        match self {
            Self::Regular => base,
            Self::GrandChampion => base * 4.0,
        }
    }

    pub fn display_size(self, base: f32) -> f32 {
        match self {
            Self::Regular => base,
            Self::GrandChampion => base * 4.0,
        }
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct EnemySpawnerPosition(pub Vec2);
