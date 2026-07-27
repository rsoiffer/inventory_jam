use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub struct DebugStatsProtocolPlugin;

impl Plugin for DebugStatsProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<ServerDebugStats>().replicate();
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub struct ServerDebugStats {
    pub tick_hz: f32,
    pub players: u32,
    pub enemies: u32,
}
