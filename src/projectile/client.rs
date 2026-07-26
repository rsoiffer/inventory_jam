use crate::app::AppState;
use crate::projectile::debug::fire_debug_projectile_client;
use crate::projectile::shared::simulate_server_projectiles;
use crate::shared::FixedGameplaySet;
use bevy::prelude::*;

pub struct ProjectileClientPlugin;

impl Plugin for ProjectileClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (fire_debug_projectile_client, simulate_server_projectiles)
                .chain()
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(AppState::Playing)),
        );
    }
}
