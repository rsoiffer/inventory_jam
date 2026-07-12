//! Application construction and launch-mode setup.

mod appstate;
mod args;
mod username;

pub use appstate::{AppState, LaunchConfig, LaunchMode, config_from_env};
pub use username::LocalUsername;

pub(crate) use username::{
    MAX_USERNAME_LEN, client_id_from_username, temporary_username, validate_username,
};
