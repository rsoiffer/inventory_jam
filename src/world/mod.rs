#[cfg(feature = "server")]
pub(crate) mod api;
pub mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
mod server;

use bevy::app::{App, Plugin};
pub use protocol::GlobalAristeia;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::WorldProtocolPlugin);
        #[cfg(feature = "gui")]
        app.add_plugins(render::WorldRenderPlugin);
        #[cfg(feature = "server")]
        app.add_plugins(server::WorldServerPlugin);
    }
}
