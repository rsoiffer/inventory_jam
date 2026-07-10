use core::net::{Ipv4Addr, SocketAddr};

use async_compat::Compat;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::tasks::block_on;
use lightyear::core::time::Instant;
use lightyear::netcode::NetcodeServer;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

use super::shared::SharedSettings;

/// Server transport backend and bind configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerTransports {
    #[cfg(feature = "udp")]
    /// UDP transport listening on a local port.
    Udp {
        /// Port the server binds for UDP traffic.
        local_port: u16,
    },
    /// WebTransport transport with a generated certificate.
    WebTransport {
        /// Port the server binds for WebTransport traffic.
        local_port: u16,
        /// Certificate settings used by the WebTransport endpoint.
        certificate: WebTransportCertificateSettings,
    },
}

/// Marker component that expands into the Lightyear server entity configuration.
#[derive(Component, Debug)]
#[component(on_add = NetworkServer::on_add)]
pub struct NetworkServer {
    /// Optional incoming link conditioner applied to client links.
    pub conditioner: Option<RecvLinkConditioner>,
    /// Transport backend used by the server.
    pub transport: ServerTransports,
    /// Shared protocol and authentication settings.
    pub shared: SharedSettings,
}

/// Link conditioner copied onto newly-created server-side client links.
#[derive(Component, Clone, Debug)]
pub(crate) struct ServerLinkConditioner(Option<RecvLinkConditioner>);

impl NetworkServer {
    /// Converts the setup marker into Lightyear server components.
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let entity = context.entity;
        world.commands().queue(move |world: &mut World| -> Result {
            let mut entity_mut = world.entity_mut(entity);
            let Some(settings) = entity_mut.take::<Self>() else {
                return Ok(());
            };
            entity_mut.insert((
                Name::from("Server"),
                ServerLinkConditioner(settings.conditioner),
            ));

            entity_mut.insert(NetcodeServer::new(NetcodeConfig {
                protocol_id: settings.shared.protocol_id,
                private_key: settings.shared.private_key,
                ..Default::default()
            }));

            match settings.transport {
                #[cfg(feature = "udp")]
                ServerTransports::Udp { local_port } => {
                    let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), local_port);
                    info!("Starting UDP server on {server_addr}");
                    entity_mut.insert((LocalAddr(server_addr), ServerUdpIo::default()));
                }
                ServerTransports::WebTransport {
                    local_port,
                    certificate,
                } => {
                    let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), local_port);
                    info!("Starting WebTransport server on {server_addr}");
                    let identity = webtransport_identity(&certificate)?;
                    entity_mut.insert((
                        LocalAddr(server_addr),
                        WebTransportServerIo {
                            certificate: identity,
                        },
                    ));
                }
            }
            Ok(())
        });
    }
}

/// Applies the server's link conditioner to each newly-created client link.
pub fn apply_server_link_conditioner(
    trigger: On<Add, LinkOf>,
    mut links: Query<(&LinkOf, &mut Link)>,
    servers: Query<&ServerLinkConditioner, With<Server>>,
) {
    let Ok((link_of, mut link)) = links.get_mut(trigger.entity) else {
        return;
    };
    let Ok(server_conditioner) = servers.get(link_of.server) else {
        return;
    };
    if link.recv.conditioner.is_some() {
        return;
    }
    let Some(conditioner) = server_conditioner.0.clone() else {
        return;
    };

    let queued_packets: Vec<_> = link.recv.drain().collect();
    link.recv.conditioner = Some(conditioner);
    for packet in queued_packets {
        link.recv.push(packet, Instant::now());
    }
}

/// Settings for loading the server's WebTransport certificate from PEM files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebTransportCertificateSettings {
    /// PEM-encoded certificate chain path.
    pub cert: String,
    /// PEM-encoded private key path.
    pub key: String,
}

/// Builds the WebTransport identity and logs its certificate digest.
fn webtransport_identity(settings: &WebTransportCertificateSettings) -> Result<Identity, String> {
    info!(
        "Loading WebTransport certificate from {} and key from {}",
        settings.cert, settings.key
    );
    let identity = block_on(Compat::new(Identity::load_pemfiles(
        &settings.cert,
        &settings.key,
    )))
    .map_err(|e| format!("failed to load WebTransport certificate files: {e}"))?;
    let digest = identity.certificate_chain().as_slice()[0].hash();
    info!("WebTransport certificate digest: {digest}");
    Ok(identity)
}
