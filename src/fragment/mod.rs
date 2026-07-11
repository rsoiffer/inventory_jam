use bevy::prelude::*;

#[cfg(feature = "client")]
pub(crate) mod client;

pub mod protocol;

#[cfg(feature = "gui")]
pub(crate) mod render;

#[cfg(feature = "server")]
pub(crate) mod server;

pub mod shared;

/// Must be installed on every peer so protocol registration remains identical.
pub struct FragmentProtocolPlugin;

impl Plugin for FragmentProtocolPlugin {
    fn build(&self, app: &mut App) {
        protocol::register(app);
    }
}

#[cfg(feature = "client")]
pub struct FragmentClientPlugin;

#[cfg(feature = "client")]
impl Plugin for FragmentClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(client::initialize_fragment);
        app.add_systems(FixedUpdate, client::simulate_client_fragments);
    }
}

#[cfg(feature = "server")]
pub struct FragmentServerPlugin;

#[cfg(feature = "server")]
impl Plugin for FragmentServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                server::drop_fragments_at_player,
                server::simulate_server_fragments,
            )
                .chain(),
        );
    }
}

#[cfg(feature = "gui")]
pub struct FragmentRenderPlugin;

#[cfg(feature = "gui")]
impl Plugin for FragmentRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, render::draw_fragments);
    }
}
