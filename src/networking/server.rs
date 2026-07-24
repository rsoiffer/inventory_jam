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

const WEBTRANSPORT_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIBmTCCAT6gAwIBAgIIJJPQEWfM3rswCgYIKoZIzj0EAwIwITEfMB0GA1UEAxMW
d3RyYW5zcG9ydC1zZWxmLXNpZ25lZDAeFw0yNjA3MjQxOTU0NDVaFw0yNjA4MDYx
OTU1NDVaMCExHzAdBgNVBAMTFnd0cmFuc3BvcnQtc2VsZi1zaWduZWQwWTATBgcq
hkjOPQIBBggqhkjOPQMBBwNCAARQ9s/1hOV/bHbv6wu6zRZC6k2JZtYIgBoXLnL3
CJHukqWBrpbXBN7qeE1cTWj0OJzT9GUQH2HiCuxdXasv3QrJo2AwXjA+BgNVHREE
NzA1gglsb2NhbGhvc3SHBH8AAAGHEAAAAAAAAAAAAAAAAAAAAAGCECoucHIuZWRn
ZWdhcC5uZXQwDAYDVR0TAQH/BAIwADAOBgNVHQ8BAf8EBAMCB4AwCgYIKoZIzj0E
AwIDSQAwRgIhALXBw82PWpES9TNVT9zAiPmf/AtoCdNVM5GgjuCD0miBAiEA4PmU
SY140TOjGG4iTqnAwyRrFhmwUlmukMugloOuGDQ=
-----END CERTIFICATE-----
";

const WEBTRANSPORT_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgXmmF+9OEB5gKY5aJ
vgixGEefFipV7Iry2pEexF70amGhRANCAARQ9s/1hOV/bHbv6wu6zRZC6k2JZtYI
gBoXLnL3CJHukqWBrpbXBN7qeE1cTWj0OJzT9GUQH2HiCuxdXasv3QrJ
-----END PRIVATE KEY-----
";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerTransports {
    #[cfg(feature = "udp")]
    Udp {
        local_port: u16,
    },
    WebTransport {
        local_port: u16,
    },
}

#[derive(Component, Debug)]
#[component(on_add = NetworkServer::on_add)]
pub struct NetworkServer {
    pub conditioner: Option<RecvLinkConditioner>,
    pub transport: ServerTransports,
    pub shared: SharedSettings,
}

#[derive(Component, Clone, Debug)]
pub struct ServerLinkConditioner(Option<RecvLinkConditioner>);

impl NetworkServer {
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
                ServerTransports::WebTransport { local_port } => {
                    let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), local_port);
                    info!("Starting WebTransport server on {server_addr}");
                    let identity = webtransport_identity()?;
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

fn webtransport_identity() -> Result<Identity, String> {
    let dir = std::env::temp_dir();
    let cert_path = dir.join("inventory-jam-webtransport-cert.pem");
    let key_path = dir.join("inventory-jam-webtransport-key.pem");
    std::fs::write(&cert_path, WEBTRANSPORT_CERT_PEM)
        .map_err(|e| format!("failed to write WebTransport cert: {e}"))?;
    std::fs::write(&key_path, WEBTRANSPORT_KEY_PEM)
        .map_err(|e| format!("failed to write WebTransport key: {e}"))?;
    let identity = block_on(Compat::new(Identity::load_pemfiles(&cert_path, &key_path)))
        .map_err(|e| format!("failed to load WebTransport certificate: {e}"))?;
    let digest = identity.certificate_chain().as_slice()[0].hash();
    info!("WebTransport certificate digest: {digest}");
    Ok(identity)
}
