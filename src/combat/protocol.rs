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

    pub fn observe(self, high_water: &mut u32, duration: f32) -> Option<f32> {
        let delta = self.sequence.wrapping_sub(*high_water);
        if delta > 0 && delta < u32::MAX / 2 {
            *high_water = self.sequence;
            Some(duration)
        } else {
            None
        }
    }
}
