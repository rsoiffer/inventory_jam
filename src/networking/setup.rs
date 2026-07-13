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
use crate::app::{AppState, LaunchMode, StartGame};
#[cfg(feature = "client")]
use crate::app::{LocalUsername, client_id_from_username, machine_local_username};

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
#[derive(Resource)]
struct NetworkingRuntime {
    certificate_digest: String,
}

#[cfg(feature = "client")]
#[derive(Resource, Clone)]
struct ClientConnection {
    client_id: u64,
    config: NetworkingConfig,
}

#[cfg(feature = "client")]
#[derive(Resource)]
struct KickoffDiscovery;

#[cfg(feature = "server")]
#[derive(Component)]
struct ServerStarted;

#[derive(Resource, Default)]
pub struct ConnectionStatus {
    pub message: String,
}

pub fn configure_networking(app: &mut App) {
    app.init_resource::<ConnectionStatus>();
    app.add_systems(Update, handle_start_game);

    #[cfg(feature = "client")]
    {
        app.add_plugins(ReqwestPlugin::default());
        app.add_systems(
            Update,
            (
                kickoff_discovery,
                send_due_status_requests,
                spawn_discovered_client,
                connect_client_once,
                patch_webtransport_digest,
                watch_client_connection_state,
                #[cfg(feature = "gui")]
                watch_discovery_failures,
            ),
        );
    }

    #[cfg(feature = "server")]
    app.add_systems(Update, start_pending_servers);
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

fn handle_start_game(
    mut commands: Commands,
    mut starts: MessageReader<StartGame>,
    mut status: ResMut<ConnectionStatus>,
    mut next_state: ResMut<NextState<AppState>>,
    #[cfg(feature = "client")] clients: Query<Entity, With<Client>>,
    #[cfg(feature = "client")] mut username: ResMut<LocalUsername>,
    #[cfg(feature = "server")] servers: Query<Entity, With<Server>>,
) {
    for start in starts.read() {
        match start.mode {
            #[cfg(feature = "client")]
            LaunchMode::JoinEdgegap => {
                if !clients.is_empty() {
                    status.message = "already connected".into();
                    continue;
                }
                username.0.clone_from(&start.username);
                let config = NetworkingConfig::for_client(false);
                commands.insert_resource(NetworkingRuntime {
                    certificate_digest: config.certificate_digest.clone(),
                });
                commands.insert_resource(ServerDiscovery::new());
                commands.insert_resource(ClientConnection {
                    client_id: client_id_from_username(&start.username),
                    config,
                });
                commands.insert_resource(KickoffDiscovery);
                status.message = "searching for Edgegap server...".into();
                next_state.set(AppState::Connecting);
            }
            #[cfg(feature = "client")]
            LaunchMode::JoinLocal => {
                if !clients.is_empty() {
                    status.message = "already connected".into();
                    continue;
                }
                let local_username = match machine_local_username(&start.username) {
                    Ok(name) => name,
                    Err(error) => {
                        status.message = error;
                        continue;
                    }
                };
                username.0.clone_from(&local_username);
                let config = NetworkingConfig::for_client(true);
                commands.insert_resource(NetworkingRuntime {
                    certificate_digest: config.certificate_digest.clone(),
                });
                info!("connecting client to {}", config.server_addr);
                status.message = format!("connecting to {}...", config.server_addr);
                next_state.set(AppState::Connecting);
                spawn_client(
                    &mut commands,
                    client_id_from_username(&local_username),
                    config.server_addr,
                    &config,
                    link_conditioner(),
                );
            }
            #[cfg(feature = "server")]
            LaunchMode::HostLocal | LaunchMode::DedicatedServer => {
                if !servers.is_empty() {
                    status.message = "server already running".into();
                    continue;
                }
                let config = NetworkingConfig::for_server();
                info!(
                    "starting server transport {:?} bind port {} public addr {} edgegap {}",
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
                status.message = format!("server running on port {}", config.server_port);
                next_state.set(AppState::Hosting);
                commands.spawn((
                    Name::new("Network Server"),
                    NetworkServer {
                        conditioner: link_conditioner(),
                        transport: server_transport(&config),
                        shared: SHARED_SETTINGS,
                    },
                ));
            }
            LaunchMode::Menu => {}
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
}

#[cfg(feature = "client")]
fn kickoff_discovery(
    mut commands: Commands,
    kickoff: Option<Res<KickoffDiscovery>>,
    discovery: Option<ResMut<ServerDiscovery>>,
    client: bevy_mod_reqwest::BevyReqwest,
) {
    if kickoff.is_none() {
        return;
    }
    commands.remove_resource::<KickoffDiscovery>();
    start_server_discovery(discovery, client);
}

#[cfg(feature = "client")]
fn spawn_discovered_client(
    mut commands: Commands,
    discovery: Option<Res<ServerDiscovery>>,
    connection: Option<Res<ClientConnection>>,
    clients: Query<Entity, With<Client>>,
    mut status: ResMut<ConnectionStatus>,
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
            status.message = format!("connecting to {server_addr}...");
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
            status.message = format!("server discovery failed: {error}");
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
    commands.spawn((
        Name::new("Network Client"),
        NetworkClient {
            client_id,
            client_port: CLIENT_PORT,
            server_addr,
            conditioner,
            transport: client_transport(config),
            certificate_digest: config.certificate_digest.clone(),
            shared: SHARED_SETTINGS,
        },
    ));
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
fn start_pending_servers(
    mut commands: Commands,
    servers: Query<Entity, (With<Server>, Without<ServerStarted>)>,
) {
    for entity in &servers {
        commands.trigger(Start { entity });
        commands.entity(entity).insert(ServerStarted);
    }
}

#[cfg(feature = "client")]
fn connect_client_once(
    mut commands: Commands,
    client: Query<Entity, (With<Client>, Without<Connected>)>,
    mut connecting: Local<Option<Entity>>,
) {
    let Some(entity) = client.iter().next() else {
        return;
    };
    if *connecting == Some(entity) {
        return;
    }
    commands.trigger(Connect { entity });
    *connecting = Some(entity);
}

#[cfg(feature = "client")]
fn patch_webtransport_digest(
    config: Option<Res<NetworkingRuntime>>,
    mut clients: Query<&mut WebTransportClientIo, With<Client>>,
) {
    let Some(config) = config else {
        return;
    };
    if config.certificate_digest.is_empty() || clients.is_empty() {
        return;
    }
    #[cfg(not(target_family = "wasm"))]
    {
        for mut io in &mut clients {
            if io.certificate_digest != config.certificate_digest {
                io.certificate_digest.clone_from(&config.certificate_digest);
            }
        }
    }
    #[cfg(target_family = "wasm")]
    {
        let _ = &mut clients;
    }
}

#[cfg(feature = "client")]
fn watch_client_connection_state(
    connected: Query<(), (With<Client>, With<Connected>)>,
    mut status: ResMut<ConnectionStatus>,
    mut next_state: ResMut<NextState<AppState>>,
    current: Res<State<AppState>>,
    mut was_connected: Local<bool>,
) {
    let is_connected = !connected.is_empty();
    if is_connected && !*was_connected {
        status.message = "connected".into();
        next_state.set(AppState::Playing);
    }
    if !is_connected && *was_connected && *current.get() == AppState::Playing {
        status.message = "disconnected".into();
        next_state.set(AppState::MainMenu);
    }
    *was_connected = is_connected;
}

#[cfg(all(feature = "client", feature = "gui"))]
fn watch_discovery_failures(
    discovery: Option<Res<ServerDiscovery>>,
    mut next_state: ResMut<NextState<AppState>>,
    current: Res<State<AppState>>,
) {
    if *current.get() != AppState::Connecting {
        return;
    }
    let Some(discovery) = discovery else {
        return;
    };
    if matches!(discovery.state, ServerDiscoveryState::Failed(_)) {
        next_state.set(AppState::MainMenu);
    }
}
