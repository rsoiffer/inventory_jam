//! This file contains the shared Protocol that defines the messages that can be sent between the client and server.
//!
//! You will need to define the Components, Messages and Inputs that make up the protocol.
//! You can use the `#[protocol]` attribute to specify additional behaviour:
//! - how entities contained in the message should be mapped from the remote world to the local world
//! - how the component should be synchronized between the `Confirmed` entity and the `Predicted`/`Interpolated` entity

use bevy::ecs::entity::MapEntities;
use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

// Player
/// Components spawned together for each replicated player entity.
#[derive(Bundle)]
pub(crate) struct PlayerBundle {
    /// Stable peer id for the player.
    id: PlayerId,
    /// Networked player position.
    position: PlayerPosition,
    /// Networked player display color.
    color: PlayerColor,
}

impl PlayerBundle {
    /// Creates a player bundle with a deterministic color derived from the peer id.
    pub(crate) fn new(id: PeerId, position: Vec2) -> Self {
        // Generate pseudo random color from client id.
        let h = (((id.to_bits().wrapping_mul(30)) % 360) as f32) / 360.0;
        let s = 0.8;
        let l = 0.5;
        let color = Color::hsl(h, s, l);
        Self {
            id: PlayerId(id),
            position: PlayerPosition(position),
            color: PlayerColor(color),
        }
    }
}

// Components

/// Replicated id of the peer that owns a player entity.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(PeerId);

/// Replicated world-space player position.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct PlayerPosition(pub Vec2);

impl Ease for PlayerPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

/// Replicated player color used by the renderer.
#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlayerColor(pub(crate) Color);

/// Example replicated component containing an entity reference.
///
/// Entity references need `MapEntities` so server-world entities can be mapped
/// to their client-world equivalents.
#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlayerParent(Entity);

impl MapEntities for PlayerParent {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.0 = entity_mapper.get_mapped(self.0);
    }
}

// Channels
/// Reliable ordered channel used by the example message.
pub struct Channel1;

// Messages

/// Example server-to-client message payload.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Message1(pub usize);

// Inputs

/// Directional movement input captured from keyboard state.
#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone, Reflect)]
pub struct Direction {
    /// Whether movement is requested upward.
    pub(crate) up: bool,
    /// Whether movement is requested downward.
    pub(crate) down: bool,
    /// Whether movement is requested leftward.
    pub(crate) left: bool,
    /// Whether movement is requested rightward.
    pub(crate) right: bool,
}

impl Direction {
    /// Returns true when no directional buttons are pressed.
    pub(crate) fn is_none(&self) -> bool {
        !self.up && !self.down && !self.left && !self.right
    }
}

/// Input variants sent from clients to the server.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Reflect)]
pub enum Inputs {
    /// Directional keyboard movement.
    Direction(Direction),
}

impl Default for Inputs {
    fn default() -> Self {
        Self::Direction(Direction::default())
    }
}

impl MapEntities for Inputs {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {}
}

// Protocol
/// Registers replicated components, messages, inputs, and channels.
#[derive(Clone)]
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // messages
        app.register_message::<Message1>()
            .add_direction(NetworkDirection::ServerToClient);

        // inputs
        app.add_plugins(input::native::InputPlugin::<Inputs>::default());
        // components
        app.component::<PlayerId>().replicate();

        app.component::<PlayerPosition>()
            .replicate()
            .predict()
            .add_linear_interpolation();

        app.component::<PlayerColor>().replicate();

        // channels
        app.add_channel::<Channel1>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);
    }
}
