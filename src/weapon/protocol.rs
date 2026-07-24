use bevy::prelude::*;
use lightyear::{core::tick::TickDuration, prelude::*};
use serde::{Deserialize, Serialize};

pub struct WeaponProtocolPlugin;

impl Plugin for WeaponProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<WeaponCooldown>().replicate().predict();
    }
}

/// Stable gameplay identifier replicated over the network.
pub type WeaponId = u16;

#[derive(
    Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Deref, DerefMut, Default,
)]
pub struct WeaponCooldown {
    pub cooldown_tick: Tick,
}

impl WeaponCooldown {
    pub fn is_ready(&self, current_tick: Tick) -> bool {
        self.cooldown_tick < current_tick
    }

    pub fn restart(
        &mut self,
        local_timeline: &LocalTimeline,
        tick_duration: &TickDuration,
        attacks_per_second: f32,
    ) {
        let ticks_per_attack = 1.0 / (attacks_per_second * tick_duration.0.as_secs_f32());
        self.cooldown_tick = local_timeline.tick() + Tick(ticks_per_attack.ceil() as u32);
    }
}
