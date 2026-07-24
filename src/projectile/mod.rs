#[cfg(feature = "client")]
mod client;
mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
mod server;
mod shared;

use bevy::app::{App, Plugin};

pub use protocol::{PlayerProjectile, ProjectileLifetime};
pub use shared::SpawnProjectile;
pub use shared::projectile_spawn_position;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::ProjectileProtocolPlugin);

        #[cfg(feature = "client")]
        app.add_plugins(client::ProjectileClientPlugin);

        #[cfg(feature = "server")]
        app.add_plugins(server::ProjectileServerPlugin);

        #[cfg(feature = "gui")]
        app.add_plugins(render::ProjectileRenderPlugin);
    }
}
