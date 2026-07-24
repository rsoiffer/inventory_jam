#[cfg(feature = "gui")]
mod help_text;

#[cfg(feature = "gui")]
mod menu;

#[cfg(feature = "gui")]
mod mobile_controls;

#[cfg(all(feature = "gui", target_family = "wasm"))]
mod browser_username;

use bevy::app::{App, Plugin};

#[cfg(feature = "gui")]
use crate::app::LaunchMode;

#[cfg(feature = "gui")]
use help_text::HelpTextPlugin;

#[cfg(feature = "gui")]
use menu::MenuPlugin;

#[cfg(feature = "gui")]
use mobile_controls::MobileControlsPlugin;

#[cfg(feature = "gui")]
pub use mobile_controls::TouchControlsEnabled;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "gui")]
        {
            use crate::app;

            app.add_plugins(HelpTextPlugin);
            app.add_plugins(MobileControlsPlugin);

            let launch_mode = app::config_from_env();

            if launch_mode.mode != LaunchMode::DedicatedServer {
                app.add_plugins(MenuPlugin);
            }
        }
    }
}
