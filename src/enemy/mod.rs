mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
mod server;
pub mod shared;

use bevy::app::{App, Plugin};

pub use protocol::{EnemyHealth, EnemyPosition};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::EnemyProtocolPlugin);

        #[cfg(feature = "server")]
        app.add_plugins(server::EnemyServerPlugin);

        #[cfg(feature = "gui")]
        app.add_plugins(render::EnemyRenderPlugin);
    }
}
