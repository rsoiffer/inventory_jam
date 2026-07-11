use bevy::prelude::*;
use lightyear::prelude::AppComponentExt;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Fragment {
    pub origin: Vec2,
}

pub fn register(app: &mut App) {
    app.component::<Fragment>().replicate();
}
