use bevy::prelude::*;
use lightyear::prelude::AppComponentExt;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct PlayerProjectile {
    pub origin: Vec2,
    pub direction: Vec2,
    pub speed_per_tick: f32,
}

pub fn register(app: &mut App) {
    app.component::<PlayerProjectile>().replicate();
}
