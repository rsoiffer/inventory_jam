use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub struct CombatProtocolPlugin;

impl Plugin for CombatProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<HitFlash>().replicate().predict();
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct HitFlash {
    pub sequence: u32,
}

impl HitFlash {
    pub fn trigger(&mut self) {
        self.sequence = self.sequence.wrapping_add(1);
    }
}
