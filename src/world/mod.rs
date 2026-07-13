#[cfg(feature = "gui")]
mod render;

use bevy::app::{App, Plugin};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "gui")]
        app.add_plugins(render::WorldRenderPlugin);
    }
}
