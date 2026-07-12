//! Low-level networking transport, connection setup, and Edgegap hosting.

#[cfg(feature = "client")]
mod client;
mod edgegap;
#[cfg(feature = "server")]
mod server;
mod setup;
mod shared;

#[cfg(feature = "server")]
pub use server::apply_server_link_conditioner;
pub use setup::{ConnectionStatus, configure_networking};
pub use shared::FIXED_TIMESTEP_HZ;
#[cfg(feature = "server")]
pub use shared::SEND_INTERVAL;
