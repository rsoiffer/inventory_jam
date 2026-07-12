pub mod protocol;

#[cfg(feature = "gui")]
pub(crate) mod render;

#[cfg(feature = "server")]
pub(crate) mod server;

/// Installs only client-side fragment presentation behavior.
#[cfg(feature = "client")]
pub struct EnemyClientPlugin;
