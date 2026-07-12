use bevy::prelude::{Component, Deref, DerefMut};
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
pub enum GameRooms {
    Arena,
    Pit,
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct GameRoom {
    pub room: GameRooms,
}
