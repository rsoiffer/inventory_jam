use bevy::prelude::*;

use crate::app::AppState;
use crate::projectile::debug::fire_debug_projectiles_server;
use crate::projectile::shared::simulate_server_projectiles;
use crate::shared::FixedGameplaySet;

pub struct ProjectileServerPlugin;

impl Plugin for ProjectileServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (fire_debug_projectiles_server, simulate_server_projectiles)
                .chain()
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(AppState::Hosting)),
        );
    }
}
