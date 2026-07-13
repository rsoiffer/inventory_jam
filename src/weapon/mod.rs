use bevy::prelude::*;

#[cfg(feature = "client")]
mod client;

pub mod protocol;

#[cfg(feature = "gui")]
mod render;

#[cfg(feature = "server")]
pub(crate) mod server;

pub mod shared;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(protocol::WeaponProtocolPlugin);

        #[cfg(feature = "client")]
        app.add_plugins(client::WeaponClientPlugin);

        #[cfg(feature = "server")]
        app.add_plugins(server::WeaponServerPlugin);

        #[cfg(feature = "gui")]
        app.add_plugins(render::WeaponRenderPlugin);
    }
}
