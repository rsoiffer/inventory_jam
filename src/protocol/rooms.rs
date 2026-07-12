use crate::shared::{ARENA_WORLD_BOUNDS, SHOP_WORLD_BOUNDS};
use bevy::math::Rect;
use bevy::prelude::{Component, Deref, DerefMut};
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
pub enum GameRooms {
    Arena,
    Pit,
}

#[derive(
    Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect, Deref, DerefMut,
)]
pub struct GameRoom {
    pub room: GameRooms,
}

impl GameRoom {
    pub fn bounds(&self) -> Rect {
        match self.room {
            GameRooms::Arena => ARENA_WORLD_BOUNDS,
            GameRooms::Pit => SHOP_WORLD_BOUNDS,
        }
    }
}
