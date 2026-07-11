use bevy::prelude::*;

/// Public request accepted by the Fragment feature.
///
/// Other gameplay features can request a pool without knowing how fragments
/// are placed, replicated, collected, animated, or despawned.
#[derive(Message, Clone, Copy, Debug)]
pub struct SpawnFragmentPool {
    pub center: Vec2,
    pub count: u16,
}

impl SpawnFragmentPool {
    pub const fn new(center: Vec2, count: u16) -> Self {
        Self { center, count }
    }
}
