use crate::protocol::rooms::GameRoom;
#[cfg(feature = "server")]
use crate::protocol::rooms::GameRooms;
use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::app::temporary_username;
#[cfg(feature = "server")]
use crate::combat::HitFlash;

pub struct PlayerProtocolPlugin;

impl Plugin for PlayerProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<PlayerId>().replicate();

        app.component::<PlayerPosition>()
            .replicate()
            .predict()
            .add_linear_interpolation();

        app.component::<PlayerAimDirection>().replicate().predict();

        app.component::<PlayerColor>().replicate();

        app.component::<GameRoom>().replicate();

        app.component::<PlayerUsername>().replicate();

        app.component::<PlayerHealth>().replicate().predict();

        app.component::<PlayerAristeia>().replicate().predict();
    }
}

#[cfg(feature = "server")]
#[derive(Bundle)]
pub(crate) struct PlayerBundle {
    id: PlayerId,
    position: PlayerPosition,
    color: PlayerColor,
    aim_direction: PlayerAimDirection,
    game_room: GameRoom,
    username: PlayerUsername,
    health: PlayerHealth,
    aristeia: PlayerAristeia,
    hit_flash: HitFlash,
    projectile_buffer: crate::projectile::protocol::ProjectileBuffer,
}

#[cfg(feature = "server")]
impl PlayerBundle {
    pub(crate) fn new(
        id: PeerId,
        position: Vec2,
        maximum_health: u32,
        projectile_buffer_capacity: usize,
    ) -> Self {
        let h = (((id.to_bits().wrapping_mul(30)) % 360) as f32) / 360.0;
        let color = Color::hsl(h, 0.8, 0.5);

        Self {
            id: PlayerId(id),
            position: PlayerPosition(position),
            color: PlayerColor(color),
            aim_direction: PlayerAimDirection::default(),
            game_room: GameRoom {
                room: GameRooms::Safezone,
            },
            username: PlayerUsername(temporary_username(id.to_bits())),
            health: PlayerHealth::new(maximum_health),
            aristeia: PlayerAristeia::default(),
            hit_flash: HitFlash::default(),
            projectile_buffer: crate::projectile::protocol::ProjectileBuffer::new(
                projectile_buffer_capacity,
            ),
        }
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Copy)]
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

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Deref)]
pub struct PlayerUsername(pub String);

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlayerHealth {
    pub current: u32,
    pub maximum: u32,
}

impl PlayerHealth {
    pub const fn new(maximum: u32) -> Self {
        Self {
            current: maximum,
            maximum,
        }
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub(crate) struct PlayerAristeia {
    pub current: u32,
    pub progress: u32,
    pub remaining_seconds: f32,
    pub maximum_seconds: f32,
}

impl PlayerAristeia {
    pub fn remaining_fraction(&self) -> f32 {
        if self.maximum_seconds <= 0.0 {
            return 0.0;
        }

        (self.remaining_seconds / self.maximum_seconds).clamp(0.0, 1.0)
    }
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct PlayerVisual;

#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct LocalAimInput(pub Vec2);

impl Default for LocalAimInput {
    fn default() -> Self {
        Self(Vec2::X)
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct SmoothedAimDirection(pub Vec2);

impl Default for SmoothedAimDirection {
    fn default() -> Self {
        Self(Vec2::X)
    }
}
