use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::rooms::GameRooms;

pub struct PortalProtocolPlugin;

impl Plugin for PortalProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<PortalPosition>()
            .replicate()
            .add_linear_interpolation();
        app.component::<PortalKind>().replicate();
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortalKind {
    ToArena,
    ToSafezone,
}

impl PortalKind {
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
pub struct PortalPosition(pub Vec2);

impl Ease for PortalPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PortalPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}
