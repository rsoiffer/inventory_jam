use crate::edgegap;
use bevy::prelude::*;
use core::net::SocketAddr;
use lightyear::link::RecvLinkConditioner;
use lightyear::prelude::client::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use lightyear_examples_common::cli::{Cli, Mode};
use lightyear_examples_common::client::{ClientTransports, ExampleClient};
use lightyear_examples_common::server::{
    ExampleServer, ServerTransports, WebTransportCertificateSettings,
};
use lightyear_examples_common::shared::{CLIENT_PORT, SERVER_ADDR, SERVER_PORT, SHARED_SETTINGS};

#[cfg(feature = "server-discovery")]
use crate::server_discovery::{USE_SERVER_DISCOVERY, discover_latest_server, resolve_server_addr};

const CERT_DIGEST: &str = "1c2066ff65d1ad7a83a1e06b5a5c08167498b24e946d2f30852e87cdd250622d";
const LOCAL_SERVER_ADDR: SocketAddr = SERVER_ADDR;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    Udp,
    WebTransport,
}

#[derive(Debug, Clone)]
pub struct NetworkingConfig {
    pub transport: TransportMode,
    pub server_port: u16,
    pub server_addr: SocketAddr,
    pub certificate_digest: String,
}

impl NetworkingConfig {
    pub fn from_cli(cli: &Cli) -> Self {
        match cli.mode {
            #[cfg(feature = "client")]
            Some(Mode::Client { .. }) => Self::for_client(),
            #[cfg(feature = "server")]
            Some(Mode::Server) => Self::for_server(),
            #[cfg(all(feature = "client", feature = "server"))]
            Some(Mode::HostClient { .. }) => Self {
                transport: TransportMode::Udp,
                server_port: SERVER_PORT,
                server_addr: SERVER_ADDR,
                certificate_digest: String::new(),
            },
            _ => Self::for_client(),
        }
    }

    fn for_client() -> Self {
        #[cfg(feature = "server-discovery")]
        if USE_SERVER_DISCOVERY {
            return Self {
                transport: TransportMode::WebTransport,
                server_port: LOCAL_SERVER_ADDR.port(),
                server_addr: LOCAL_SERVER_ADDR,
                certificate_digest: CERT_DIGEST.into(),
            };
        }
        Self {
            transport: TransportMode::Udp,
            server_port: LOCAL_SERVER_ADDR.port(),
            server_addr: LOCAL_SERVER_ADDR,
            certificate_digest: String::new(),
        }
    }

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

    #[cfg(feature = "server-discovery")]
    pub fn with_server_discovery(mut self, cli: &Cli) -> Result<Self, String> {
        if !USE_SERVER_DISCOVERY || !matches!(cli.mode, Some(Mode::Client { .. })) {
            return Ok(self);
        }
        let server = discover_latest_server()?;
        self.server_addr = resolve_server_addr(&server.host, server.port).map_err(|e| {
            format!(
                "failed to resolve {}:{}: {e}",
                server.host, server.port
            )
        })?;
        info!(
            "discovered server {}:{} -> {}",
            server.host,
            server.port,
            self.server_addr
        );
        Ok(self)
    }

    #[cfg(not(feature = "server-discovery"))]
    pub fn with_server_discovery(self, _cli: &Cli) -> Result<Self, String> {
        Ok(self)
    }
}

pub fn spawn_connections(app: &mut App, cli: &Cli) -> Result<(), String> {
    let config = NetworkingConfig::from_cli(cli).with_server_discovery(cli)?;
    app.insert_resource(NetworkingRuntime {
        certificate_digest: config.certificate_digest.clone(),
    });

    let conditioner = LinkConditionerConfig::average_condition().half();

    match cli.mode {
        #[cfg(feature = "client")]
        Some(Mode::Client { client_id }) => {
            let Some(client_id) = client_id else {
                return Err("client mode requires a client id (`-c ID`)".into());
            };
            app.world_mut().spawn(ExampleClient {
                client_id,
                client_port: CLIENT_PORT,
                server_addr: config.server_addr,
                conditioner: Some(RecvLinkConditioner::new(conditioner.clone())),
                transport: client_transport(&config),
                shared: SHARED_SETTINGS,
            });
            app.add_systems(Update, patch_webtransport_digest);
            app.add_systems(Startup, connect_client);
        }
        #[cfg(feature = "server")]
        Some(Mode::Server) => {
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
        #[cfg(all(feature = "client", feature = "server"))]
        Some(Mode::HostClient { .. }) => {
            let server = app
                .world_mut()
                .spawn(ExampleServer {
                    conditioner: Some(RecvLinkConditioner::new(conditioner.clone())),
                    transport: ServerTransports::Udp {
                        local_port: SERVER_PORT,
                    },
                    shared: SHARED_SETTINGS,
                })
                .id();
            app.world_mut().spawn((
                Client::default(),
                Name::new("HostClient"),
                LinkOf { server },
            ));
            app.add_systems(Startup, (start_server, connect_client).chain());
        }
        _ => {}
    }
    Ok(())
}

fn client_transport(config: &NetworkingConfig) -> ClientTransports {
    match config.transport {
        TransportMode::WebTransport => ClientTransports::WebTransport,
        TransportMode::Udp => ClientTransports::Udp,
    }
}

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
        TransportMode::Udp => ServerTransports::Udp {
            local_port: config.server_port,
        },
    }
}

#[derive(Resource, Clone)]
pub struct NetworkingRuntime {
    pub certificate_digest: String,
}

fn start_server(mut commands: Commands, server: Single<Entity, With<Server>>) {
    commands.trigger(Start {
        entity: server.into_inner(),
    });
}

fn connect_client(mut commands: Commands, client: Single<Entity, With<Client>>) {
    commands.trigger(Connect {
        entity: client.into_inner(),
    });
}

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
    for mut io in clients.iter_mut() {
        io.certificate_digest = config.certificate_digest.clone();
    }
    *done = true;
}
