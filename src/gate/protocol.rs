use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::rooms::GameRooms;

pub struct GateProtocolPlugin;

impl Plugin for GateProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<GatePosition>()
            .replicate()
            .add_linear_interpolation();
        app.component::<GateKind>().replicate();
        app.component::<GateOpen>().replicate();
        app.component::<GateProgress>().replicate();
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateKind {
    ToArena,
    ToSafezone,
}

impl GateKind {
    pub fn source_room(self) -> GameRooms {
        match self {
            Self::ToArena => GameRooms::Safezone,
            Self::ToSafezone => GameRooms::Arena,
        }
    }

    pub fn destination_room(self) -> GameRooms {
        match self {
            Self::ToArena => GameRooms::Arena,
            Self::ToSafezone => GameRooms::Safezone,
        }
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct GatePosition(pub Vec2);

impl Ease for GatePosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            GatePosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

/// Arena escape gates start closed; safezone→arena gates stay open.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateOpen(pub bool);

/// Charge from 0.0 to 1.0 for arena gates. Safezone gates keep 1.0.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct GateProgress(pub f32);
