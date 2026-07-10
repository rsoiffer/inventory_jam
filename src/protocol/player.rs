//! Replicated player identity, position, and color components.

use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

/// Bundle of replicated components spawned for each connected player.
#[derive(Bundle)]
pub(crate) struct PlayerBundle {
    /// Network peer that owns this player.
    id: PlayerId,
    /// World-space position replicated to clients.
    position: PlayerPosition,
    /// Display color derived from the peer id.
    color: PlayerColor,
}

impl PlayerBundle {
    /// Creates a player bundle with a color derived from `id`.
    pub(crate) fn new(id: PeerId, position: Vec2) -> Self {
        let h = f32::from((id.to_bits().wrapping_mul(30) % 360) as u16) / 360.0;
        let color = Color::hsl(h, 0.8, 0.5);

        Self {
            id: PlayerId(id),
            position: PlayerPosition(position),
            color: PlayerColor(color),
        }
    }
}

/// Stable network identity for a player entity.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PlayerId(pub PeerId);

/// Replicated player position used for prediction and interpolation.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct PlayerPosition(pub Vec2);

impl Ease for PlayerPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| Self(Vec2::lerp(start.0, end.0, t)))
    }
}

/// Replicated display color for a player entity.
#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlayerColor(pub(crate) Color);

/// Registers player components for replication, prediction, and interpolation.
pub fn register(app: &mut App) {
    app.component::<PlayerId>().replicate();

    app.component::<PlayerPosition>()
        .replicate()
        .predict()
        .add_linear_interpolation();

    app.component::<PlayerColor>().replicate();
}
