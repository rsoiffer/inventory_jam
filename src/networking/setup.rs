use bevy::prelude::*;
#[cfg(feature = "client")]
use bevy_mod_reqwest::ReqwestPlugin;
use core::net::SocketAddr;
use lightyear::link::RecvLinkConditioner;
#[cfg(feature = "link_conditioner")]
use lightyear::prelude::LinkConditionerConfig;
#[cfg(feature = "client")]
use lightyear::prelude::client::*;
#[cfg(feature = "server")]
use lightyear::prelude::server::*;

use super::RunMode;
#[cfg(feature = "client")]
use super::client::{ClientTransports, NetworkClient};
#[cfg(feature = "server")]
use super::edgegap;
#[cfg(feature = "client")]
use super::edgegap::{
    ServerDiscovery, ServerDiscoveryState, send_due_status_requests, start_server_discovery,
};
#[cfg(feature = "server")]
use super::server::{NetworkServer, ServerTransports};
#[cfg(feature = "client")]
use super::shared::CLIENT_PORT;
#[cfg(any(feature = "client", feature = "server"))]
use super::shared::SERVER_ADDR;
#[cfg(feature = "server")]
use super::shared::SERVER_PORT;
use super::shared::SHARED_SETTINGS;

#[cfg(feature = "client")]
const CERT_DIGEST: &str = "18b16f92178824528aabb1c4274a0f247d0dec2c6f755e152739ff2fe343ec7c";
#[cfg(feature = "client")]
const LOCAL_SERVER_ADDR: SocketAddr = SERVER_ADDR;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    Udp,
    WebTransport,
}

#[derive(Debug, Clone)]
pub struct NetworkingConfig {
    pub transport: TransportMode,
    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    pub server_port: u16,
    pub server_addr: SocketAddr,
    #[cfg(feature = "client")]
    pub certificate_digest: String,
}

impl NetworkingConfig {
    pub fn for_mode(mode: RunMode) -> Self {
        match mode {
            #[cfg(feature = "client")]
            RunMode::Client {
                use_local_server, ..
            } => Self::for_client(use_local_server),
            #[cfg(feature = "server")]
            RunMode::Server => Self::for_server(),
        }
    }

    #[cfg(feature = "client")]
    fn for_client(use_local_server: bool) -> Self {
        if use_local_server {
            return Self {
                transport: TransportMode::Udp,
                server_port: LOCAL_SERVER_ADDR.port(),
                server_addr: LOCAL_SERVER_ADDR,
                certificate_digest: String::new(),
            };
        }
        Self {
            transport: TransportMode::WebTransport,
            server_port: LOCAL_SERVER_ADDR.port(),
            server_addr: LOCAL_SERVER_ADDR,
            certificate_digest: CERT_DIGEST.into(),
        }
    }

    #[cfg(feature = "server")]
    fn for_server() -> Self {
        let (server_port, server_addr) = if let Some(deployment) = edgegap::deployment_info() {
            (
                deployment.internal_port,
                edgegap::host_port_from_deployment(&deployment),
            )
        } else {
            (SERVER_PORT, SocketAddr::new(SERVER_ADDR.ip(), SERVER_PORT))
        };
        let transport = if edgegap::is_edgegap_server() {
            TransportMode::WebTransport
        } else {
            TransportMode::Udp
        };
        Self {
            transport,
            server_port,
            server_addr,
            #[cfg(feature = "client")]
            certificate_digest: String::new(),
        }
    }
}

#[cfg(feature = "client")]
#[derive(Resource, Clone)]
pub struct NetworkingRuntime {
    pub certificate_digest: String,
}

#[cfg(feature = "client")]
#[derive(Resource, Clone)]
struct ClientConnection {
    client_id: u64,
    config: NetworkingConfig,
}

pub fn spawn_connections(app: &mut App, mode: RunMode) {
    let config = NetworkingConfig::for_mode(mode);
    #[cfg(feature = "client")]
    app.insert_resource(NetworkingRuntime {
        certificate_digest: config.certificate_digest.clone(),
    });

    let conditioner = link_conditioner();

    match mode {
        #[cfg(feature = "client")]
        RunMode::Client {
            client_id,
            use_local_server,
        } => {
            if use_local_server {
                info!("connecting client to {}", config.server_addr);
                spawn_client(
                    &mut app.world_mut().commands(),
                    client_id,
                    config.server_addr,
                    &config,
                    conditioner,
                );
                app.add_systems(Update, (connect_client_once, patch_webtransport_digest));
            } else {
                app.add_plugins(ReqwestPlugin::default());
                app.insert_resource(ServerDiscovery::new());
                app.insert_resource(ClientConnection { client_id, config });
                app.add_systems(Startup, start_server_discovery);
                app.add_systems(
                    Update,
                    (
                        send_due_status_requests,
                        spawn_discovered_client,
                        connect_client_once,
                        patch_webtransport_digest,
                    ),
                );
            }
        }
        #[cfg(feature = "server")]
        RunMode::Server => {
            info!(
                "server networking config transport {:?} bind port {} public addr {} edgegap {}",
                config.transport,
                config.server_port,
                config.server_addr,
                edgegap::is_edgegap_server()
            );
            if let Some(deployment) = edgegap::deployment_info() {
                info!(
                    "edgegap deployment {} public {}:{} internal {}",
                    deployment.request_id,
                    deployment.public_ip,
                    deployment.public_port,
                    deployment.internal_port
                );
            }
            app.world_mut().spawn(NetworkServer {
                conditioner,
                transport: server_transport(&config),
                shared: SHARED_SETTINGS,
            });
            app.add_systems(Startup, start_server);
        }
    }
}

fn link_conditioner() -> Option<RecvLinkConditioner> {
    #[cfg(feature = "link_conditioner")]
    {
        Some(RecvLinkConditioner::new(
            LinkConditionerConfig::average_condition().half(),
        ))
    }
    #[cfg(not(feature = "link_conditioner"))]
    {
        None
    }
}

#[cfg(feature = "client")]
fn spawn_discovered_client(
    mut commands: Commands,
    discovery: Option<Res<ServerDiscovery>>,
    connection: Option<Res<ClientConnection>>,
    clients: Query<Entity, With<Client>>,
    mut logged_failure: Local<bool>,
) {
    if !clients.is_empty() {
        return;
    }
    let (Some(discovery), Some(connection)) = (discovery, connection) else {
        return;
    };
    match &discovery.state {
        ServerDiscoveryState::Discovering => {}
        ServerDiscoveryState::Ready(server_addr) => {
            info!("connecting client to {server_addr}");
            spawn_client(
                &mut commands,
                connection.client_id,
                *server_addr,
                &connection.config,
                link_conditioner(),
            );
            commands.remove_resource::<ClientConnection>();
            commands.remove_resource::<ServerDiscovery>();
        }
        ServerDiscoveryState::Failed(error) => {
            if !*logged_failure {
                error!("server discovery failed: {error}");
                *logged_failure = true;
            }
        }
    }
}

#[cfg(feature = "client")]
fn spawn_client(
    commands: &mut Commands,
    client_id: u64,
    server_addr: SocketAddr,
    config: &NetworkingConfig,
    conditioner: Option<RecvLinkConditioner>,
) {
    commands.spawn(NetworkClient {
        client_id,
        client_port: CLIENT_PORT,
        server_addr,
        conditioner,
        transport: client_transport(config),
        certificate_digest: config.certificate_digest.clone(),
        shared: SHARED_SETTINGS,
    });
}

#[cfg(feature = "client")]
const fn client_transport(config: &NetworkingConfig) -> ClientTransports {
    match config.transport {
        TransportMode::WebTransport => ClientTransports::WebTransport,
        #[cfg(all(not(target_family = "wasm"), feature = "udp"))]
        TransportMode::Udp => ClientTransports::Udp,
        #[cfg(any(target_family = "wasm", not(feature = "udp")))]
        TransportMode::Udp => ClientTransports::WebTransport,
    }
}

#[cfg(feature = "server")]
fn server_transport(config: &NetworkingConfig) -> ServerTransports {
    match config.transport {
        TransportMode::WebTransport => ServerTransports::WebTransport {
            local_port: config.server_port,
        },
        #[cfg(feature = "udp")]
        TransportMode::Udp => ServerTransports::Udp {
            local_port: config.server_port,
        },
        #[cfg(not(feature = "udp"))]
        TransportMode::Udp => ServerTransports::WebTransport {
            local_port: config.server_port,
        },
    }
}

#[cfg(feature = "server")]
fn start_server(mut commands: Commands, server: Single<Entity, With<Server>>) {
    commands.trigger(Start {
        entity: server.into_inner(),
    });
}

#[cfg(feature = "client")]
fn connect_client_once(
    mut commands: Commands,
    client: Query<Entity, With<Client>>,
    mut connected: Local<bool>,
) {
    if *connected || client.is_empty() {
        return;
    }
    let Some(entity) = client.iter().next() else {
        return;
    };
    commands.trigger(Connect { entity });
    *connected = true;
}

#[cfg(feature = "client")]
fn patch_webtransport_digest(
    config: Res<NetworkingRuntime>,
    mut clients: Query<&mut WebTransportClientIo, With<Client>>,
    mut done: Local<bool>,
) {
    if *done || config.certificate_digest.is_empty() {
        *done = true;
        return;
    }
    if clients.is_empty() {
        return;
    }
    #[cfg(not(target_family = "wasm"))]
    {
        for mut io in &mut clients {
            io.certificate_digest.clone_from(&config.certificate_digest);
        }
    }
    *done = true;
}
