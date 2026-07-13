use crate::app::AppState;
use crate::projectile::client::{
    initialize_projectile, initialize_projectile_impact, simulate_client_projectiles,
};
#[cfg(feature = "gui")]
use bevy::prelude::*;

#[cfg(feature = "client")]
mod client;

pub mod protocol;

#[cfg(feature = "gui")]
pub(crate) mod render;

#[cfg(feature = "server")]
pub(crate) mod server;

pub mod shared;

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
    }
}
