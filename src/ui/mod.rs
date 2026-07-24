#[cfg(feature = "gui")]
mod help_text;

#[cfg(feature = "gui")]
mod menu;

use bevy::app::{App, Plugin};

#[cfg(feature = "gui")]
use crate::app::LaunchMode;

#[cfg(feature = "gui")]
use help_text::HelpTextPlugin;

#[cfg(feature = "gui")]
use menu::MenuPlugin;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "gui")]
        {
            use crate::app;

            app.add_plugins(HelpTextPlugin);

            let launch_mode = app::config_from_env();

            if launch_mode.mode != LaunchMode::DedicatedServer {
                app.add_plugins(MenuPlugin);
            }
        }
    }
}
