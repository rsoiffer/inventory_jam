use crate::app::AppState;
use crate::projectile::client::{
    initialize_projectile, initialize_projectile_impact, simulate_client_projectiles,
};
use crate::projectile::protocol::ProjectileImpact;
#[cfg(feature = "gui")]
use bevy::prelude::*;
use lightyear::prelude::AppComponentExt;

#[cfg(feature = "client")]
mod client;

mod protocol;

#[cfg(feature = "gui")]
pub(crate) mod render;

#[cfg(feature = "server")]
pub(crate) mod server;

pub mod shared;

pub use crate::projectile::protocol::PlayerProjectile;

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
        render::register(app);
        app.component::<PlayerProjectile>().replicate();
        app.component::<ProjectileImpact>().replicate();
    }
}
