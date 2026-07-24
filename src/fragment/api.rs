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

pub(crate) fn split_fragment_values(total_value: u32, drop_entity_count: u16) -> Vec<u32> {
    if total_value == 0 || drop_entity_count == 0 {
        return Vec::new();
    }

    let entity_count = (drop_entity_count as u32).min(total_value);
    let base = total_value / entity_count;
    let remainder = total_value % entity_count;
    (0..entity_count)
        .map(|index| base + u32::from(index < remainder))
        .collect()
}
