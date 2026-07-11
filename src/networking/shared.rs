use core::net::{IpAddr, Ipv4Addr, SocketAddr};
#[cfg(feature = "server")]
use core::time::Duration;

pub const FIXED_TIMESTEP_HZ: f64 = 64.0;
pub const SERVER_PORT: u16 = 5888;
#[cfg(feature = "client")]
pub const CLIENT_PORT: u16 = 0;
pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), SERVER_PORT);
#[cfg(feature = "server")]
pub const SEND_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Copy, Clone, Debug)]
pub struct SharedSettings {
    pub protocol_id: u64,
    pub private_key: [u8; 32],
}

pub const SHARED_SETTINGS: SharedSettings = SharedSettings {
    protocol_id: 0,
    private_key: [0; 32],
};
