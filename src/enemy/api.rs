use bevy::prelude::*;

use crate::enemy::EnemyKind;

#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct SpawnEnemy {
    pub position: Vec2,
    pub kind: EnemyKind,
}

impl SpawnEnemy {
    pub const fn regular(position: Vec2) -> Self {
        Self {
            position,
            kind: EnemyKind::Regular,
        }
    }

    pub const fn grand_champion(position: Vec2) -> Self {
        Self {
            position,
            kind: EnemyKind::GrandChampion,
        }
    }
}
