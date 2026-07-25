use bevy::prelude::*;

/// Public request accepted by the Fragment feature.
///
/// Fragments always spawn in the arena.
#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct SpawnFragmentPool {
    pub center: Vec2,
    pub total_value: u32,
}

impl SpawnFragmentPool {
    pub const fn new(center: Vec2, total_value: u32) -> Self {
        Self {
            center,
            total_value,
        }
    }
}
