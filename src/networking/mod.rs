//! Low-level networking transport and connection setup.

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "server")]
mod server;
mod setup;
mod shared;

#[cfg(feature = "server")]
pub use server::apply_server_link_conditioner;
pub use setup::configure_networking;
pub use shared::FIXED_TIMESTEP_HZ;
#[cfg(feature = "server")]
pub use shared::SEND_INTERVAL;
