#[cfg(feature = "client")]
mod client;
mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
mod server;
pub mod shared;

use crate::app::AppState;
use crate::projectile::client::{
    initialize_projectile, initialize_projectile_impact, simulate_client_projectiles,
};
use crate::projectile::protocol::ProjectileImpact;
use crate::projectile::server::{expire_server_impacts, simulate_server_projectiles};
use crate::shared::FixedGameplaySet;
#[cfg(feature = "gui")]
use bevy::prelude::*;
use lightyear::prelude::AppComponentExt;

pub use protocol::PlayerProjectile;

#[cfg(feature = "gui")]
pub struct ProjectileRenderPlugin;

#[cfg(feature = "gui")]
impl Plugin for ProjectileRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(initialize_projectile);
        app.add_observer(initialize_projectile_impact);
        app.add_systems(
            FixedUpdate,
            simulate_client_projectiles
                .chain()
                .run_if(in_state(AppState::Playing)),
        );
        app.add_systems(
            FixedUpdate,
            (simulate_server_projectiles, expire_server_impacts)
                .chain()
                .in_set(FixedGameplaySet::ProjectileSimulation)
                .run_if(in_state(AppState::Hosting)),
        );
        render::register(app);
        app.component::<PlayerProjectile>().replicate();
        app.component::<ProjectileImpact>().replicate();
    }
}
