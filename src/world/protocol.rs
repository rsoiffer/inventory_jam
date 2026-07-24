use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub struct WorldProtocolPlugin;

impl Plugin for WorldProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<GlobalAristeia>().replicate();
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalAristeia {
    pub current: u32,
    pub maximum: u32,
}

impl GlobalAristeia {
    pub const fn new(maximum: u32) -> Self {
        Self {
            current: 0,
            maximum,
        }
    }

    #[allow(unused)]
    pub fn fraction(&self) -> f32 {
        if self.maximum == 0 {
            0.0
        } else {
            (self.current as f32 / self.maximum as f32).clamp(0.0, 1.0)
        }
    }
}
