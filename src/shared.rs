use crate::config::{SEND_INTERVAL, SHARED_PRIVATE_KEY, SHARED_PROTOCOL_ID};
use crate::protocol::*;
use bevy::prelude::*;
use lightyear::prelude::*;

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);
    }
}

pub struct SharedSettings {
    pub protocol_id: u64,
    pub private_key: [u8; 32],
}

impl Default for SharedSettings {
    fn default() -> Self {
        Self {
            protocol_id: SHARED_PROTOCOL_ID,
            private_key: SHARED_PRIVATE_KEY,
        }
    }
}

pub fn shared_movement_behaviour(mut position: Mut<PlayerPosition>, input: &Inputs) {
    const MOVE_SPEED: f32 = 10.0;
    position.0 += input.0 * MOVE_SPEED;
}

pub fn replication_metadata() -> ReplicationMetadata {
    ReplicationMetadata::new(SEND_INTERVAL)
}
