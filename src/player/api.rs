use bevy::prelude::*;
use lightyear::core::id::PeerId;
use serde::{Deserialize, Serialize};

/// Public request accepted by the Player feature.
#[derive(Message, Clone, Serialize, Deserialize, Debug)]
pub(crate) struct AddKillsToAristeia(pub PeerId, pub u16);

impl AddKillsToAristeia {
    pub const fn new(owner_id: PeerId, number_of_kills: u16) -> Self {
        Self(owner_id, number_of_kills)
    }
}
