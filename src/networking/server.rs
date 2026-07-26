use core::net::{Ipv4Addr, SocketAddr};

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use lightyear::core::time::Instant;
use lightyear::netcode::NetcodeServer;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

use super::shared::SharedSettings;

#[derive(Component, Debug)]
#[component(on_add = NetworkServer::on_add)]
pub struct NetworkServer {
    pub conditioner: Option<RecvLinkConditioner>,
    pub local_port: u16,
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

            let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), settings.local_port);
            info!("Starting UDP server on {server_addr}");
            entity_mut.insert((LocalAddr(server_addr), ServerUdpIo::default()));
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
