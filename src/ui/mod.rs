#[cfg(feature = "gui")]
mod camera;

#[cfg(feature = "gui")]
mod hud;

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
use camera::CameraPlugin;

#[cfg(feature = "gui")]
use hud::HudPlugin;

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

            app.add_plugins(CameraPlugin);
            app.add_plugins(HudPlugin);
            app.add_plugins(MobileControlsPlugin);

            let launch_mode = app::config_from_env();

            if launch_mode.mode != LaunchMode::DedicatedServer {
                app.add_plugins(MenuPlugin);
            }
        }
    }
}
