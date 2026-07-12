use crate::protocol::rooms::{GameRoom, GameRooms};
use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Bundle)]
pub(crate) struct PlayerBundle {
    id: PlayerId,
    position: PlayerPosition,
    color: PlayerColor,
    aim_direction: PlayerAimDirection,
    game_room: GameRoom,
}

impl PlayerBundle {
    pub(crate) fn new(id: PeerId, position: Vec2) -> Self {
        let h = (((id.to_bits().wrapping_mul(30)) % 360) as f32) / 360.0;
        let color = Color::hsl(h, 0.8, 0.5);

        Self {
            id: PlayerId(id),
            position: PlayerPosition(position),
            color: PlayerColor(color),
            aim_direction: PlayerAimDirection::default(),
            game_room: GameRoom {
                room: GameRooms::Arena,
            },
        }
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(pub PeerId);

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct PlayerPosition(pub Vec2);

impl Ease for PlayerPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct PlayerAimDirection(pub Vec2);

impl Default for PlayerAimDirection {
    fn default() -> Self {
        let default_direction = Vec2 { x: 5.0, y: 4.0 };
        Self(default_direction) // Defaults to [5,4] for debug since it's a weird direction
    }
}

#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlayerColor(pub(crate) Color);

pub fn register(app: &mut App) {
    app.component::<PlayerId>().replicate();

    app.component::<PlayerPosition>()
        .replicate()
        .predict()
        .add_linear_interpolation();

    app.component::<PlayerAimDirection>().replicate().predict();

    app.component::<PlayerColor>().replicate();

    app.component::<GameRoom>().replicate();
}
