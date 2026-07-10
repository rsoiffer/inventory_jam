use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DebugServerMessage(pub usize);

pub fn register(app: &mut App) {
    app.register_message::<DebugServerMessage>()
        .add_direction(NetworkDirection::ServerToClient);
}
