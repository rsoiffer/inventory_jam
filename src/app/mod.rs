//! Application construction and launch-mode setup.

mod appstate;
mod args;
mod username;

pub(crate) use appstate::config_from_env;
pub(crate) use appstate::game_is_active;
pub use appstate::{ClientState, LaunchMode, LaunchPlugin, ServerState, StartGame};
pub use username::LocalUsername;

#[cfg(feature = "gui")]
pub(crate) use username::MAX_USERNAME_LEN;
#[cfg(feature = "client")]
pub(crate) use username::client_id_from_username;
#[cfg(feature = "client")]
pub(crate) use username::machine_local_username;
#[cfg(feature = "server")]
pub(crate) use username::{temporary_username, validate_username};
