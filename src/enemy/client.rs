use bevy::prelude::*;

use crate::{
    app::ClientState,
    enemy::shared::{fire_enemy_projectiles, simulate_enemy_ai},
    shared::FixedGameplaySet,
};

pub struct EnemyClientPlugin;

impl Plugin for EnemyClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (simulate_enemy_ai, fire_enemy_projectiles)
                .chain()
                .in_set(FixedGameplaySet::Enemy)
                .run_if(in_state(ClientState::Playing)),
        );
    }
}
