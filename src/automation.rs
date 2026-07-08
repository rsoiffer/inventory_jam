use bevy::diagnostic::DiagnosticsPlugin;
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
#[cfg(feature = "client")]
use core::net::{Ipv4Addr, SocketAddr};
#[cfg(feature = "client")]
use lightyear::prelude::client::{NetcodeClient, NetcodeConfig};
#[cfg(all(feature = "client", feature = "webtransport"))]
use lightyear::prelude::client::WebTransportClientIo;
#[cfg(all(feature = "client", not(feature = "webtransport"), feature = "udp"))]
use lightyear::prelude::UdpIo;
#[cfg(feature = "client")]
use lightyear::prelude::{
    Authentication, Client, Connect, Link, LinkOf, PeerAddr, PredictionManager,
};
#[cfg(feature = "server")]
use lightyear::connection::server::Start;
#[cfg(feature = "server")]
use lightyear::prelude::server::{NetcodeConfig as ServerNetcodeConfig, NetcodeServer};
#[cfg(all(feature = "server", feature = "webtransport", not(target_family = "wasm")))]
use lightyear::prelude::server::WebTransportServerIo;
#[cfg(all(feature = "server", not(feature = "webtransport"), feature = "udp"))]
use lightyear::prelude::server::ServerUdpIo;
#[cfg(any(feature = "client", feature = "server"))]
use lightyear::prelude::LocalAddr;

use crate::shared;

#[derive(Resource, Clone, Copy, Debug)]
pub struct ClientStartupConfig {
    pub client_id: u64,
    pub host_server: Option<Entity>,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct ServerStartupConfig {
    pub auto_spawn: bool,
}

impl Default for ServerStartupConfig {
    fn default() -> Self {
        Self { auto_spawn: true }
    }
}

pub fn headless_enabled() -> bool {
    std::env::var("LIGHTYEAR_HEADLESS")
        .map(|value| value != "0")
        .unwrap_or(false)
}

pub fn build_base_app() -> App {
    let mut app = App::new();
    if headless_enabled() {
        app.add_plugins((
            MinimalPlugins,
            LogPlugin {
                level: Level::INFO,
                ..default()
            },
            StatesPlugin,
            DiagnosticsPlugin,
        ));
    } else {
        app.add_plugins(DefaultPlugins);
    }
    app
}

/// Spawn the entity that listens for incoming connections and start it.
///
/// Must be called after the `ProtocolPlugin` (via `SharedPlugin`) has been added, and before
/// [`spawn_host_client`] if this is a host-server (the server needs to be started before the
/// in-process host client tries to connect to it).
#[cfg(feature = "server")]
pub fn spawn_server(app: &mut App) -> Entity {
    let server = app
        .world_mut()
        .spawn((
            NetcodeServer::new(ServerNetcodeConfig::default()),
            LocalAddr(shared::SERVER_ADDR),
            #[cfg(all(feature = "webtransport", not(target_family = "wasm")))]
            WebTransportServerIo {
                certificate: shared::webtransport_self_signed_certificate(),
            },
            #[cfg(all(not(feature = "webtransport"), feature = "udp"))]
            ServerUdpIo::default(),
        ))
        .id();
    app.world_mut().trigger(Start { entity: server });
    server
}

/// Spawn a dedicated client entity that connects to `SERVER_ADDR` over the network, and start
/// connecting it.
///
/// Must be called after the `ProtocolPlugin` (via `SharedPlugin`) has been added.
#[cfg(feature = "client")]
pub fn spawn_client(app: &mut App, client_id: u64) -> Entity {
    let auth = Authentication::Manual {
        server_addr: shared::SERVER_ADDR,
        client_id,
        private_key: shared::PRIVATE_KEY,
        protocol_id: shared::PROTOCOL_ID,
    };
    let netcode_config = NetcodeConfig {
        // Make sure that the server times out clients when their connection is closed.
        client_timeout_secs: 3,
        token_expire_secs: -1,
        ..default()
    };
    let netcode_client =
        NetcodeClient::new(auth, netcode_config).expect("manual netcode authentication is valid");
    let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0);

    let entity = app
        .world_mut()
        .spawn((
            Client::default(),
            Link::new(None),
            LocalAddr(client_addr),
            PeerAddr(shared::SERVER_ADDR),
            PredictionManager::default(),
            netcode_client,
            #[cfg(feature = "webtransport")]
            WebTransportClientIo {
                certificate_digest: String::new(),
            },
            #[cfg(all(not(feature = "webtransport"), feature = "udp"))]
            UdpIo::default(),
        ))
        .id();
    app.world_mut().trigger(Connect { entity });
    entity
}

/// Spawn the in-process client entity used by host-server mode: it connects to `server` directly
/// via [`LinkOf`] instead of over a socket. `server` must already have been started (see
/// [`spawn_server`]).
#[cfg(all(feature = "client", feature = "server"))]
pub fn spawn_host_client(app: &mut App, server: Entity) -> Entity {
    let entity = app
        .world_mut()
        .spawn((Client::default(), Name::new("HostClient"), LinkOf { server }))
        .id();
    app.world_mut().trigger(Connect { entity });
    entity
}
