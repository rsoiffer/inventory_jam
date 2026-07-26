pub(crate) mod api;
#[cfg(feature = "client")]
mod client;
mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
pub(crate) mod server;
pub mod shared;

use bevy::app::{App, Plugin};

pub use protocol::{EnemyHealth, EnemyIdentity, EnemyKind, EnemyPosition, EnemySpawnerPosition};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::EnemyProtocolPlugin);

        #[cfg(feature = "client")]
        app.add_plugins(client::EnemyClientPlugin);

        #[cfg(feature = "server")]
        app.add_plugins(server::EnemyServerPlugin);

        #[cfg(feature = "gui")]
        app.add_plugins(render::EnemyRenderPlugin);
    }
}
