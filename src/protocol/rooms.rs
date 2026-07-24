use crate::settings::WorldSettings;
use bevy::math::Rect;
use bevy::prelude::{Component, Deref, DerefMut};
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
pub enum GameRooms {
    Arena,
    Safezone,
}

#[derive(
    Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect, Deref, DerefMut,
)]
pub(crate) struct GameRoom {
    pub room: GameRooms,
}

impl GameRoom {
    pub fn bounds(&self, settings: &WorldSettings) -> Rect {
        match self.room {
            GameRooms::Arena => settings.arena_bounds(),
            GameRooms::Safezone => settings.safezone_bounds(),
        }
    }
}
