use bevy::prelude::*;

use crate::enemy::EnemyKind;

#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct SpawnEnemy {
    pub position: Vec2,
    pub kind: EnemyKind,
    pub spawner: Option<Entity>,
    pub tier_index: Option<u8>,
}

impl SpawnEnemy {
    pub const fn regular(position: Vec2, spawner: Entity, tier_index: u8) -> Self {
        Self {
            position,
            kind: EnemyKind::Regular,
            spawner: Some(spawner),
            tier_index: Some(tier_index),
        }
    }

    pub const fn grand_champion(position: Vec2) -> Self {
        Self {
            position,
            kind: EnemyKind::GrandChampion,
            spawner: None,
            tier_index: None,
        }
    }
}
