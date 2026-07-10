use core::net::{Ipv4Addr, SocketAddr};

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use lightyear::netcode::NetcodeClient;
use lightyear::netcode::client_plugin::NetcodeConfig;
use lightyear::prelude::client::*;
use lightyear::prelude::*;

use crate::net::shared::SharedSettings;

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
#[component(on_add = ExampleClient::on_add)]
pub struct ExampleClient {
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
    /// Shared protocol and authentication settings.
    pub shared: SharedSettings,
}

impl ExampleClient {
    /// Converts the setup marker into Lightyear client components.
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let entity = context.entity;
        world.commands().queue(move |world: &mut World| -> Result {
            let mut entity_mut = world.entity_mut(entity);
            let Some(settings) = entity_mut.take::<ExampleClient>() else {
                return Ok(());
            };
            let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), settings.client_port);
            entity_mut.insert((
                Client::default(),
                Link::new(settings.conditioner.clone()),
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
                client_timeout_secs: 3,
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
                    let certificate_digest = {
                        #[cfg(target_family = "wasm")]
                        {
                            browser_certificate_digest()
                        }
                        #[cfg(not(target_family = "wasm"))]
                        {
                            String::new()
                        }
                    };
                    entity_mut.insert(WebTransportClientIo { certificate_digest });
                }
            }
            Ok(())
        });
    }
}

#[cfg(target_family = "wasm")]
/// Reads the WebTransport certificate digest from browser config or the bundled fallback.
fn browser_certificate_digest() -> String {
    browser_digest_override().unwrap_or_else(|| {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/certificates/digest.txt"))
            .trim()
            .to_string()
    })
}

#[cfg(target_family = "wasm")]
/// Reads a digest override from `#<digest>` or `window.CERT_DIGEST`.
fn browser_digest_override() -> Option<String> {
    let window = web_sys::window()?;
    let hash = window.location().hash().ok()?;
    let hash = hash.trim_start_matches('#').trim();
    if !hash.is_empty() {
        return Some(hash.to_string());
    }
    js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("CERT_DIGEST"))
        .ok()
        .and_then(|value| value.as_string())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
