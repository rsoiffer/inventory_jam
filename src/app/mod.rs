//! Application construction and launch-mode setup.

mod appstate;
mod args;
mod username;

pub(crate) use appstate::game_is_active;
pub use appstate::{AppState, LaunchMode, LaunchPlugin, StartGame, config_from_env};
pub use username::LocalUsername;

#[cfg(any(feature = "client", feature = "server"))]
pub(crate) use username::client_id_from_username;
