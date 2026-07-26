use bevy::prelude::*;

#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct AddGlobalAristeia(pub u32);

#[derive(Message, Clone, Copy, Debug, Default)]
pub(crate) struct GrandChampionDefeated;
