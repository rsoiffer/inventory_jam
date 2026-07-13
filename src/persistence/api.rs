use bevy::prelude::*;

#[cfg(feature = "server")]
#[derive(Component, Debug, Clone)]
pub struct PersistenceIdentity {
    pub username: String,
}

#[cfg(feature = "server")]
#[derive(Component, Debug, Default)]
pub struct PersistenceReady;

#[cfg(feature = "server")]
#[derive(Component, Debug)]
pub struct PersistenceLoading {
    pub request_id: u64,
    pub username: String,
}

/// Request to credit fragments to a persistence-ready player.
///
/// Gameplay systems write this message; persistence applies it optimistically
/// and queues the durable transaction for the next database flush.
#[cfg(feature = "server")]
#[derive(Message, Clone, Copy, Debug)]
pub struct AddFragments {
    pub player: Entity,
    pub amount: u32,
}

#[cfg(feature = "server")]
impl AddFragments {
    pub const fn new(player: Entity, amount: u32) -> Self {
        Self { player, amount }
    }
}
