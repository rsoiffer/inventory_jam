use bevy::prelude::*;

#[cfg(feature = "server")]
#[derive(Component, Debug, Clone)]
pub struct PersistenceIdentity {
    pub username: String,
}

#[cfg(feature = "server")]
#[derive(Component, Debug, Default)]
pub struct PersistenceReady;

#[cfg(feature = "server")]
#[derive(Component, Debug, Clone)]
pub struct PersistenceLoading {
    pub username: String,
}
