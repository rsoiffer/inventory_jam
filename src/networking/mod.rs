//! Low-level networking transport, connection setup, and Edgegap hosting.
//!
//! Treat this module as a mini-library: edit [`crate::protocol`], [`crate::client`],
//! and [`crate::server`] for gameplay networking changes.

#[cfg(feature = "client")]
/// Client transport marker and Lightyear client entity setup.
mod client;
/// Edgegap server env helpers and client deployment discovery.
mod edgegap;
#[cfg(feature = "server")]
/// Server transport marker and Lightyear server entity setup.
mod server;
/// Client/server connection spawning and runtime networking config.
mod setup;
/// Shared ports, tick rate, and netcode settings.
mod shared;

pub use setup::spawn_connections;
pub use shared::FIXED_TIMESTEP_HZ;
#[cfg(feature = "server")]
pub use shared::SEND_INTERVAL;
#[cfg(feature = "server")]
pub use server::apply_server_link_conditioner;
