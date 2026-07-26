use bevy::prelude::*;
use lightyear::core::id::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Message, Clone, Serialize, Deserialize, Debug)]
pub(crate) struct AddAristeiaPoints(pub PeerId, pub u32);

impl AddAristeiaPoints {
    pub const fn new(owner_id: PeerId, points: u32) -> Self {
        Self(owner_id, points)
    }
}
