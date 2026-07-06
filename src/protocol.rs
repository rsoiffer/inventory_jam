use bevy::ecs::entity::MapEntities;
use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::frame_interpolation::FrameInterpolationPlugin;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Bundle)]
pub struct PlayerBundle {
    id: PlayerId,
    position: PlayerPosition,
    color: PlayerColor,
}

impl PlayerBundle {
    pub fn new(id: PeerId, position: Vec2) -> Self {
        let h = (((id.to_bits().wrapping_mul(30)) % 360) as f32) / 360.0;
        Self {
            id: PlayerId(id),
            position: PlayerPosition(position),
            color: PlayerColor(Color::hsl(h, 0.8, 0.5)),
        }
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct PlayerId(pub PeerId);

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
#[reflect(Component)]
pub struct PlayerPosition(pub Vec2);

impl Ease for PlayerPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct PlayerColor(pub Color);

pub struct Channel1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Message1(pub usize);

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone, Reflect, Deref, DerefMut)]
pub struct Inputs(pub Vec2);

impl MapEntities for Inputs {
    fn map_entities<M: EntityMapper>(&mut self, _entity_mapper: &mut M) {}
}

#[derive(Clone)]
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PlayerId>()
            .register_type::<PlayerPosition>()
            .register_type::<PlayerColor>()
            .register_type::<Inputs>()
            .register_message::<Message1>()
            .add_direction(NetworkDirection::ServerToClient);
        app.add_plugins(input::native::InputPlugin::<Inputs>::default());
        app.component::<PlayerId>().replicate();
        app.add_plugins(FrameInterpolationPlugin::<PlayerPosition>::default());
        app.component::<PlayerPosition>()
            .replicate()
            .predict()
            .add_linear_interpolation();
        app.component::<PlayerColor>().replicate();
        app.add_channel::<Channel1>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);
    }
}
