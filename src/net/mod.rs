#[cfg(feature = "client")]
/// Client-side transport component setup.
pub mod client;
#[cfg(feature = "server")]
/// Server-side transport component setup.
pub mod server;
/// Shared networking constants and netcode settings.
pub mod shared;

#[cfg(feature = "client")]
pub use client::{ClientTransports, ExampleClient};
#[cfg(feature = "server")]
pub use server::{ExampleServer, ServerTransports, WebTransportCertificateSettings};
pub use shared::{
    CLIENT_PORT, FIXED_TIMESTEP_HZ, SEND_INTERVAL, SERVER_ADDR, SERVER_PORT, SHARED_SETTINGS,
    SharedSettings,
};
