use core::net::{Ipv4Addr, SocketAddr};

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use lightyear::netcode::NetcodeClient;
use lightyear::netcode::client_plugin::NetcodeConfig;
use lightyear::prelude::client::*;
use lightyear::prelude::*;

use super::shared::SharedSettings;

/// Client transport backend selected for a connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientTransports {
    #[cfg(all(not(target_family = "wasm"), feature = "udp"))]
    /// UDP transport for native local development.
    Udp,
    /// WebTransport transport for browser and Edgegap connections.
    WebTransport,
}

/// Marker component that expands into the Lightyear client entity configuration.
#[derive(Component, Clone, Debug)]
#[component(on_add = NetworkClient::on_add)]
pub struct NetworkClient {
    /// Stable client id used for netcode authentication.
    pub client_id: u64,
    /// Local port the client binds to.
    pub client_port: u16,
    /// Remote server address the client connects to.
    pub server_addr: SocketAddr,
    /// Optional incoming link conditioner for local testing.
    pub conditioner: Option<RecvLinkConditioner>,
    /// Transport backend used by the client.
    pub transport: ClientTransports,
    /// Certificate digest used by WebTransport clients.
    pub certificate_digest: String,
    /// Shared protocol and authentication settings.
    pub shared: SharedSettings,
}

impl NetworkClient {
    /// Converts the setup marker into Lightyear client components.
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let entity = context.entity;
        world.commands().queue(move |world: &mut World| -> Result {
            let mut entity_mut = world.entity_mut(entity);
            let Some(settings) = entity_mut.take::<Self>() else {
                return Ok(());
            };
            let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), settings.client_port);
            entity_mut.insert((
                Client::default(),
                Link::new(settings.conditioner),
                LocalAddr(client_addr),
                PeerAddr(settings.server_addr),
                PredictionManager::default(),
                Name::from("Client"),
            ));

            let auth = Authentication::Manual {
                server_addr: settings.server_addr,
                client_id: settings.client_id,
                private_key: settings.shared.private_key,
                protocol_id: settings.shared.protocol_id,
            };
            let netcode_config = NetcodeConfig {
                client_timeout_secs: 10,
                token_expire_secs: -1,
                ..default()
            };
            entity_mut.insert(NetcodeClient::new(auth, netcode_config)?);

            match settings.transport {
                #[cfg(all(not(target_family = "wasm"), feature = "udp"))]
                ClientTransports::Udp => {
                    entity_mut.insert(UdpIo::default());
                }
                ClientTransports::WebTransport => {
                    let certificate_digest = settings.certificate_digest;
                    entity_mut.insert(WebTransportClientIo { certificate_digest });
                }
            }
            Ok(())
        });
    }
}
