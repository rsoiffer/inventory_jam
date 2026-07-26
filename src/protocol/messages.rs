use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SetUsername {
    pub name: String,
}

pub fn register(app: &mut App) {
    app.register_message::<SetUsername>()
        .add_direction(NetworkDirection::ClientToServer);
}
