use bevy::prelude::*;

#[cfg(feature = "client")]
pub(crate) mod client;

pub mod protocol;

#[cfg(feature = "gui")]
pub(crate) mod render;

#[cfg(feature = "server")]
pub(crate) mod server;

pub mod shared;

#[cfg(feature = "gui")]
pub struct ProjectileRenderPlugin;

#[cfg(feature = "gui")]
impl Plugin for ProjectileRenderPlugin {
    fn build(&self, app: &mut App) {
        render::register(app);
    }
}
