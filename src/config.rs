use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use core::time::Duration;

use bevy::prelude::*;

pub const FIXED_TIMESTEP_HZ: f64 = 64.0;
pub const SERVER_PORT: u16 = 5888;
pub const CLIENT_PORT: u16 = 0;
pub const SEND_INTERVAL: Duration = Duration::from_millis(100);

pub const SHARED_PROTOCOL_ID: u64 = 0;
pub const SHARED_PRIVATE_KEY: [u8; 32] = [0; 32];

pub fn default_server_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), SERVER_PORT)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Reflect)]
pub enum SessionMode {
    #[default]
    Host,
    Join,
}

#[derive(Clone, Debug, Resource, Reflect)]
#[reflect(Resource)]
pub struct NetworkSession {
    pub mode: SessionMode,
    #[reflect(ignore, default = "default_server_addr")]
    pub server_addr: SocketAddr,
    pub client_id: u64,
}

impl NetworkSession {
    pub fn host(client_id: u64) -> Self {
        Self {
            mode: SessionMode::Host,
            server_addr: default_server_addr(),
            client_id,
        }
    }

    pub fn join(server_addr: SocketAddr, client_id: u64) -> Self {
        Self {
            mode: SessionMode::Join,
            server_addr,
            client_id,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn default_join_server_addr() -> SocketAddr {
    let window = web_sys::window().expect("browser window");
    let hostname = window
        .location()
        .hostname()
        .unwrap_or_else(|_| "localhost".to_string());
    format!("{hostname}:{SERVER_PORT}")
        .parse()
        .unwrap_or_else(|_| default_server_addr())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn default_join_server_addr() -> SocketAddr {
    default_server_addr()
}

pub fn webtransport_certificate_digest() -> String {
    include_str!("../certificates/digest.txt")
        .trim()
        .to_string()
}

pub fn webtransport_cert_path() -> String {
    format!("{}/certificates/cert.pem", env!("CARGO_MANIFEST_DIR"))
}

pub fn webtransport_key_path() -> String {
    format!("{}/certificates/key.pem", env!("CARGO_MANIFEST_DIR"))
}
