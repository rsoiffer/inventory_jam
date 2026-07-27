use crate::app::AppState;
#[cfg(feature = "gui")]
use crate::projectile::debug::{
    PendingDebugFireRequests, fire_debug_projectile_at_cursor_client,
    queue_debug_fire_at_cursor_client,
};
// use crate::projectile::debug::fire_debug_projectile_client;
use crate::projectile::shared::simulate_server_projectiles;
use crate::shared::FixedGameplaySet;
use bevy::prelude::*;

pub struct ProjectileClientPlugin;

impl Plugin for ProjectileClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            // Automatic periodic debug fire disabled for now.
            (/* fire_debug_projectile_client, */simulate_server_projectiles)
                .chain()
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(AppState::Playing)),
        );

        #[cfg(feature = "gui")]
        {
            app.init_resource::<PendingDebugFireRequests>();
            app.add_systems(
                Update,
                queue_debug_fire_at_cursor_client.run_if(in_state(AppState::Playing)),
            );
            app.add_systems(
                FixedUpdate,
                fire_debug_projectile_at_cursor_client
                    .in_set(FixedGameplaySet::Projectile)
                    .run_if(in_state(AppState::Playing)),
            );
        }
    }
}
