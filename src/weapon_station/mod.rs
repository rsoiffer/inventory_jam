pub mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
mod server;

use bevy::app::{App, Plugin};

pub use protocol::{StationPosition, UpgradeStation, WeaponStationId};

pub struct WeaponStationPlugin;

impl Plugin for WeaponStationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::WeaponStationProtocolPlugin);

        #[cfg(feature = "server")]
        app.add_plugins(server::WeaponStationServerPlugin);

        #[cfg(feature = "gui")]
        app.add_plugins(render::WeaponStationRenderPlugin);
    }
}
