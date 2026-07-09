use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use core::time::Duration;

/// Fixed simulation tick rate shared by client and server.
pub const FIXED_TIMESTEP_HZ: f64 = 64.0;
/// Default game server port for local runs.
pub const SERVER_PORT: u16 = 5888;
/// Client bind port, with zero allowing the OS to choose an ephemeral port.
pub const CLIENT_PORT: u16 = 0;
/// Default loopback server address for local development.
pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), SERVER_PORT);
/// Interval used for replicated message sending.
pub const SEND_INTERVAL: Duration = Duration::from_millis(100);

/// Netcode settings that must match between client and server.
#[derive(Copy, Clone, Debug)]
pub struct SharedSettings {
    /// Protocol id used to reject mismatched clients.
    pub protocol_id: u64,
    /// Private key used by Lightyear netcode authentication.
    pub private_key: [u8; 32],
}

/// Default shared networking settings for this example.
pub const SHARED_SETTINGS: SharedSettings = SharedSettings {
    protocol_id: 0,
    private_key: [0; 32],
};
