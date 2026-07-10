//! Network messages exchanged between server and clients.

use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

/// Debug payload sent from the server to all clients.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DebugServerMessage(pub usize);

/// Registers protocol messages and their network directions.
pub fn register(app: &mut App) {
    app.register_message::<DebugServerMessage>()
        .add_direction(NetworkDirection::ServerToClient);
}
