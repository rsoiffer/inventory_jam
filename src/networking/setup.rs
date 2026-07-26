use bevy::prelude::*;
use core::net::SocketAddr;
use lightyear::link::RecvLinkConditioner;
#[cfg(feature = "link_conditioner")]
use lightyear::prelude::LinkConditionerConfig;
#[cfg(feature = "client")]
use lightyear::prelude::client::*;
#[cfg(feature = "server")]
use lightyear::prelude::server::*;

#[cfg(feature = "client")]
use super::client::NetworkClient;
#[cfg(feature = "server")]
use super::server::NetworkServer;
#[cfg(feature = "client")]
use super::shared::CLIENT_PORT;
#[cfg(any(feature = "client", feature = "server"))]
use super::shared::SERVER_ADDR;
#[cfg(feature = "server")]
use super::shared::SERVER_PORT;
use super::shared::SHARED_SETTINGS;
#[cfg(feature = "client")]
use crate::app::LocalUsername;
#[cfg(feature = "client")]
use crate::app::client_id_from_username;
use crate::app::{AppState, LaunchMode, StartGame};

#[cfg(feature = "client")]
const LOCAL_SERVER_ADDR: SocketAddr = SERVER_ADDR;

#[derive(Debug, Clone)]
pub struct NetworkingConfig {
    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    pub server_port: u16,
    pub server_addr: SocketAddr,
}

impl NetworkingConfig {
    #[cfg(feature = "client")]
    fn for_client() -> Self {
        Self {
            server_port: LOCAL_SERVER_ADDR.port(),
            server_addr: LOCAL_SERVER_ADDR,
        }
    }

    #[cfg(feature = "server")]
    fn for_server() -> Self {
        Self {
            server_port: SERVER_PORT,
            server_addr: SocketAddr::new(SERVER_ADDR.ip(), SERVER_PORT),
        }
    }
}

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
    app.add_systems(Update, (connect_client_once, watch_client_connection_state));

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
            LaunchMode::JoinLocal => {
                if !clients.is_empty() {
                    status.message = "already connected".into();
                    continue;
                }
                username.0.clone_from(&start.username);
                let config = NetworkingConfig::for_client();
                info!("connecting client to {}", config.server_addr);
                status.message = format!("connecting to {}...", config.server_addr);
                next_state.set(AppState::Connecting);
                spawn_client(
                    &mut commands,
                    client_id_from_username(&start.username),
                    config.server_addr,
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
                    "starting server bind port {} public addr {}",
                    config.server_port, config.server_addr,
                );
                status.message = format!("server running on port {}", config.server_port);
                next_state.set(AppState::Hosting);
                commands.spawn((
                    Name::new("Network Server"),
                    NetworkServer {
                        conditioner: link_conditioner(),
                        local_port: config.server_port,
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
fn spawn_client(
    commands: &mut Commands,
    client_id: u64,
    server_addr: SocketAddr,
    conditioner: Option<RecvLinkConditioner>,
) {
    commands.spawn((
        Name::new("Network Client"),
        NetworkClient {
            client_id,
            client_port: CLIENT_PORT,
            server_addr,
            conditioner,
            shared: SHARED_SETTINGS,
        },
    ));
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
