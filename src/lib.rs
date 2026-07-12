pub mod app;
pub mod protocol;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

pub mod enemy;
pub mod fragment;
pub mod projectile;
pub mod weapon;

#[cfg(feature = "gui")]
pub mod renderer;
#[cfg(feature = "gui")]
pub mod ui;

pub mod shared;

pub mod networking;
