pub mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
mod server;
pub mod shared;

use bevy::app::{App, Plugin};

pub use protocol::{GateKind, GateOpen, GatePosition, GateProgress};
#[cfg(feature = "server")]
pub(crate) use server::GateTeleportCooldown;

pub struct GatePlugin;

impl Plugin for GatePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::GateProtocolPlugin);

        #[cfg(feature = "server")]
        app.add_plugins(server::GateServerPlugin);

        #[cfg(feature = "gui")]
        app.add_plugins(render::GateRenderPlugin);
    }
}
