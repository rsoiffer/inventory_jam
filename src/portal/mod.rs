pub mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
mod server;
pub mod shared;

use bevy::app::{App, Plugin};

pub use protocol::{PortalKind, PortalPosition};
#[cfg(feature = "server")]
pub(crate) use server::PortalTeleportCooldown;

pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::PortalProtocolPlugin);

        #[cfg(feature = "server")]
        app.add_plugins(server::PortalServerPlugin);

        #[cfg(feature = "gui")]
        app.add_plugins(render::PortalRenderPlugin);
    }
}
