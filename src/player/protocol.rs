#[cfg(feature = "dev")]
use crate::protocol::inputs::PlayerAction;
use crate::protocol::rooms::GameRoom;
#[cfg(feature = "server")]
use crate::protocol::rooms::GameRooms;
use bevy::math::Curve;
use bevy::prelude::*;
#[cfg(feature = "dev")]
use leafwing_input_manager::action_state::ActionState;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::app::temporary_username;

pub struct PlayerProtocolPlugin;

impl Plugin for PlayerProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<PlayerId>().replicate();

        #[cfg(feature = "dev")]
        app.add_systems(PostUpdate, debug_player_position);
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

#[cfg(feature = "dev")]
#[allow(unused)]
fn debug_player_position(
    q: Query<(Entity, &PlayerPosition, &ActionState<PlayerAction>)>,
    local_timeline: Res<LocalTimeline>,
) {
    for (entity, position, actions) in q.iter() {
        let movement = actions.clamped_axis_pair(&PlayerAction::Move);

        if movement != Vec2::ZERO {
            // info!(
            //     "Player Pos: {:?} {:?} {:?}",
            //     local_timeline.tick(),
            //     position,
            //     entity
            // );
        }
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
    projectile_buffer: crate::projectile::protocol::ProjectileBuffer,
}

#[cfg(feature = "server")]
impl PlayerBundle {
    pub(crate) fn new(
        id: PeerId,
        position: Vec2,
        maximum_health: u32,
        aristeia_duration_ticks: u16,
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
            aristeia: PlayerAristeia::new(aristeia_duration_ticks),
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

// Used for Killstreak Tracking
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlayerAristeia {
    /// Current Aristeia kill count or multiplier level.
    pub current: u32,

    /// Remaining lifetime of the current Aristeia level, in fixed ticks.
    pub remaining_ticks: u16,

    /// Maximum lifetime used to calculate the UI bar fraction.
    pub maximum_ticks: u16,
}

impl PlayerAristeia {
    pub(crate) const fn new(duration_ticks: u16) -> Self {
        Self {
            current: 0,
            remaining_ticks: 0,
            maximum_ticks: duration_ticks,
        }
    }

    #[allow(unused)]
    pub fn remaining_fraction(&self) -> f32 {
        if self.maximum_ticks == 0 {
            return 0.0;
        }

        (self.remaining_ticks as f32 / self.maximum_ticks as f32).clamp(0.0, 1.0)
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
