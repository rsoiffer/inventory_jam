use bevy::prelude::*;

use crate::protocol::rooms::GameRoom;

/// Public request accepted by the Fragment feature.
///
/// Other gameplay features can request a pool without knowing how fragments
/// are placed, replicated, collected, animated, or despawned.
#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct SpawnFragmentPool {
    pub center: Vec2,
    pub total_value: u32,
    pub room: GameRoom,
}

impl SpawnFragmentPool {
    pub const fn new(center: Vec2, total_value: u32, room: GameRoom) -> Self {
        Self {
            center,
            total_value,
            room,
        }
    }
}
