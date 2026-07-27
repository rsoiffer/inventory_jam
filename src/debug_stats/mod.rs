pub mod protocol;
#[cfg(feature = "server")]
mod server;

use bevy::app::{App, Plugin};
pub use protocol::ServerDebugStats;

pub struct DebugStatsPlugin;

impl Plugin for DebugStatsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::DebugStatsProtocolPlugin);
        #[cfg(feature = "server")]
        app.add_plugins(server::DebugStatsServerPlugin);
    }
}
