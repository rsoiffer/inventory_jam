pub mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
mod server;

use bevy::app::{App, Plugin};

use crate::settings::UpgradeStatKind;

pub use protocol::{StationPosition, UpgradeStationKind, WeaponStationId};

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

pub(crate) fn upgrade_station_label(kind: UpgradeStatKind, cost: u32) -> String {
    let name = match kind {
        UpgradeStatKind::Damage => "Damage",
        UpgradeStatKind::AttackSpeed => "Attack Speed",
        UpgradeStatKind::Pierce => "Pierce",
        UpgradeStatKind::Crit => "Crit",
    };
    format!("{name} ({cost})")
}
