#[cfg(feature = "server")]
mod api;
#[cfg(feature = "client")]
mod client;
mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
mod server;
mod shared;

use bevy::app::{App, Plugin};

pub struct FragmentPlugin;

impl Plugin for FragmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::FragmentProtocolPlugin);

        #[cfg(feature = "client")]
        app.add_plugins(client::FragmentClientPlugin);

        #[cfg(feature = "server")]
        app.add_plugins(server::FragmentServerPlugin);

        #[cfg(feature = "gui")]
        app.add_plugins(render::FragmentRenderPlugin);
    }
}
