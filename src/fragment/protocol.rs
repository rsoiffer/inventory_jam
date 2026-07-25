use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub struct FragmentProtocolPlugin;

impl Plugin for FragmentProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<Fragment>().replicate();
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentPhase {
    OnGround,
    StartingPull { collector: PeerId, start_tick: Tick },
    Moving { collector: PeerId, start_tick: Tick },
    CollectingImpact { collector: PeerId, start_tick: Tick },
    Gone,
}

impl FragmentPhase {
    pub fn collector(self) -> Option<PeerId> {
        match self {
            Self::OnGround | Self::Gone => None,
            Self::StartingPull { collector, .. }
            | Self::Moving { collector, .. }
            | Self::CollectingImpact { collector, .. } => Some(collector),
        }
    }
}

/// Server owns collection and lifetime; clients may advance presentation locally.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Fragment {
    pub value: u32,
    pub phase: FragmentPhase,
    pub position: Vec2,
    pub movement: Vec2,
    pub remaining_ticks: u16,
}
