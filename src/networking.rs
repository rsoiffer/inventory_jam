use crate::app::RunMode;
use crate::edgegap;
#[cfg(any(feature = "client", feature = "server"))]
use crate::net::SERVER_ADDR;
use crate::net::SHARED_SETTINGS;
#[cfg(feature = "client")]
use crate::net::{CLIENT_PORT, ClientTransports, ExampleClient};
#[cfg(feature = "server")]
use crate::net::{ExampleServer, SERVER_PORT, ServerTransports, WebTransportCertificateSettings};
#[cfg(feature = "client")]
use crate::server_discovery::{
    ServerDiscovery, ServerDiscoveryState, USE_LOCAL_SERVER, send_due_status_requests,
    start_server_discovery,
};
use bevy::prelude::*;
#[cfg(feature = "client")]
use bevy_mod_reqwest::ReqwestPlugin;
use core::net::SocketAddr;
use lightyear::link::RecvLinkConditioner;
use lightyear::prelude::client::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

/// Pinned digest for the Edgegap WebTransport certificate used by clients.
const CERT_DIGEST: &str = "1c2066ff65d1ad7a83a1e06b5a5c08167498b24e946d2f30852e87cdd250622d";
/// Local server address used before remote discovery resolves an address.
const LOCAL_SERVER_ADDR: SocketAddr = {
    #[cfg(feature = "client")]
    {
        SERVER_ADDR
    }
    #[cfg(not(feature = "client"))]
    {
        SocketAddr::new(
            core::net::IpAddr::V4(core::net::Ipv4Addr::LOCALHOST),
            SERVER_PORT,
        )
    }
};

/// Transport backend selected for a client or server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    /// UDP sockets.
    Udp,
    /// WebTransport over HTTP/3.
    WebTransport,
}

/// Runtime-independent networking configuration.
#[derive(Debug, Clone)]
pub struct NetworkingConfig {
    /// Transport backend to spawn.
    pub transport: TransportMode,
    /// Local port used when binding a server transport.
    pub server_port: u16,
    /// Remote address clients connect to or public server address.
    pub server_addr: SocketAddr,
    /// Certificate digest clients use for WebTransport verification.
    pub certificate_digest: String,
}

impl NetworkingConfig {
    /// Builds networking configuration for the selected run mode.
    pub fn for_mode(mode: RunMode) -> Self {
        match mode {
            #[cfg(feature = "client")]
            RunMode::Client { .. } => Self::for_client(),
            #[cfg(feature = "server")]
            RunMode::Server => Self::for_server(),
        }
    }

    #[cfg(feature = "client")]
    /// Builds client-side transport configuration.
    fn for_client() -> Self {
        if USE_LOCAL_SERVER {
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
    /// Builds server-side transport configuration from Edgegap or local defaults.
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
            certificate_digest: String::new(),
        }
    }
}

/// Runtime networking data shared with follow-up systems.
#[derive(Resource, Clone)]
pub struct NetworkingRuntime {
    /// Certificate digest to patch into native WebTransport clients.
    pub certificate_digest: String,
}

#[cfg(feature = "client")]
/// Stored client connection settings used after async server discovery completes.
#[derive(Resource, Clone)]
struct ClientConnection {
    /// Client id used by netcode authentication.
    client_id: u64,
    /// Transport configuration to use when spawning the client.
    config: NetworkingConfig,
}

/// Installs client or server network entities and systems for the selected mode.
pub fn spawn_connections(app: &mut App, mode: RunMode) -> Result<(), String> {
    let config = NetworkingConfig::for_mode(mode);
    app.insert_resource(NetworkingRuntime {
        certificate_digest: config.certificate_digest.clone(),
    });

    let conditioner = LinkConditionerConfig::average_condition().half();

    match mode {
        #[cfg(feature = "client")]
        RunMode::Client { client_id } => {
            if USE_LOCAL_SERVER {
                info!("connecting client to {}", config.server_addr);
                spawn_client(
                    &mut app.world_mut().commands(),
                    client_id,
                    config.server_addr,
                    &config,
                    RecvLinkConditioner::new(conditioner),
                );
            } else {
                app.add_plugins(ReqwestPlugin::default());
                app.insert_resource(ServerDiscovery::new(client_id.to_string()));
                app.insert_resource(ClientConnection {
                    client_id,
                    config: config.clone(),
                });
                app.add_systems(Startup, start_server_discovery);
            }
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
        #[cfg(feature = "server")]
        RunMode::Server => {
            if let Some(deployment) = edgegap::deployment_info() {
                info!(
                    "edgegap deployment {} public {}:{} internal {}",
                    deployment.request_id,
                    deployment.public_ip,
                    deployment.public_port,
                    deployment.internal_port
                );
            }
            app.world_mut().spawn(ExampleServer {
                conditioner: Some(RecvLinkConditioner::new(conditioner)),
                transport: server_transport(&config),
                shared: SHARED_SETTINGS,
            });
            app.add_systems(Startup, start_server);
        }
    }
    Ok(())
}

#[cfg(feature = "client")]
/// Spawns the client once server discovery resolves a remote address.
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
                RecvLinkConditioner::new(LinkConditionerConfig::average_condition().half()),
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
/// Spawns the setup marker that expands into a Lightyear client entity.
fn spawn_client(
    commands: &mut Commands,
    client_id: u64,
    server_addr: SocketAddr,
    config: &NetworkingConfig,
    conditioner: RecvLinkConditioner,
) {
    commands.spawn(ExampleClient {
        client_id,
        client_port: CLIENT_PORT,
        server_addr,
        conditioner: Some(conditioner),
        transport: client_transport(config),
        shared: SHARED_SETTINGS,
    });
}

#[cfg(feature = "client")]
/// Converts shared networking config into a client transport enum.
fn client_transport(config: &NetworkingConfig) -> ClientTransports {
    match config.transport {
        TransportMode::WebTransport => ClientTransports::WebTransport,
        #[cfg(all(not(target_family = "wasm"), feature = "udp"))]
        TransportMode::Udp => ClientTransports::Udp,
        #[cfg(any(target_family = "wasm", not(feature = "udp")))]
        TransportMode::Udp => ClientTransports::WebTransport,
    }
}

#[cfg(feature = "server")]
/// Converts shared networking config into a server transport enum.
fn server_transport(config: &NetworkingConfig) -> ServerTransports {
    match config.transport {
        TransportMode::WebTransport => ServerTransports::WebTransport {
            local_port: config.server_port,
            certificate: WebTransportCertificateSettings::AutoSelfSigned(vec![
                "localhost".into(),
                "127.0.0.1".into(),
                "::1".into(),
            ]),
        },
        #[cfg(feature = "udp")]
        TransportMode::Udp => ServerTransports::Udp {
            local_port: config.server_port,
        },
        #[cfg(not(feature = "udp"))]
        TransportMode::Udp => ServerTransports::WebTransport {
            local_port: config.server_port,
            certificate: WebTransportCertificateSettings::AutoSelfSigned(vec![
                "localhost".into(),
                "127.0.0.1".into(),
                "::1".into(),
            ]),
        },
    }
}

#[cfg(feature = "server")]
/// Triggers Lightyear server startup after the server entity exists.
fn start_server(mut commands: Commands, server: Single<Entity, With<Server>>) {
    commands.trigger(Start {
        entity: server.into_inner(),
    });
}

/// Connects the first spawned client exactly once.
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

/// Patches the WebTransport certificate digest after Lightyear creates the IO component.
fn patch_webtransport_digest(
    config: Res<NetworkingRuntime>,
    clients: Query<&mut WebTransportClientIo, With<Client>>,
    mut done: Local<bool>,
) {
    if *done || config.certificate_digest.is_empty() {
        *done = true;
        return;
    }
    if clients.is_empty() {
        return;
    }
    #[cfg(target_family = "wasm")]
    {
        // Browser clients choose the certificate digest when the WebTransport IO is created.
    }
    #[cfg(not(target_family = "wasm"))]
    {
        let mut clients = clients;
        for mut io in clients.iter_mut() {
            io.certificate_digest = config.certificate_digest.clone();
        }
    }
    *done = true;
}
