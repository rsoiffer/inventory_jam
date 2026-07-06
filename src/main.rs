// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod audio;
mod client;
mod config;
mod debug_log;
mod launch;
mod loading;
mod menu;
mod network;
mod network_log;
mod player;
mod protocol;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod server;
mod shared;

use crate::actions::ActionsPlugin;
use crate::audio::InternalAudioPlugin;
use crate::launch::LaunchMode;
use crate::loading::LoadingPlugin;
use crate::menu::MenuPlugin;
use crate::network::NetworkPlugin;
use crate::player::PlayerPlugin;

use bevy::app::App;
use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
#[cfg(debug_assertions)]
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
#[cfg(debug_assertions)]
use bevy_inspector_egui::bevy_egui::EguiPlugin;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash, Reflect)]
enum GameState {
    #[default]
    Loading,
    Playing,
    Menu,
}

struct GamePlugin {
    launch: LaunchMode,
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        if !launch::is_dedicated_server(&self.launch) {
            app.init_state::<GameState>()
                .register_type::<GameState>()
                .add_plugins((
                LoadingPlugin,
                MenuPlugin,
                ActionsPlugin,
                InternalAudioPlugin,
                PlayerPlugin,
            ));

            #[cfg(debug_assertions)]
            {
                app.add_plugins((
                    EguiPlugin::default(),
                    WorldInspectorPlugin::new(),
                    FrameTimeDiagnosticsPlugin::default(),
                ));
            }
        }

        app.add_plugins(NetworkPlugin {
            launch: self.launch.clone(),
        });
    }
}

fn main() {
    let launch = launch::launch_mode();
    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    if launch::is_dedicated_server(&launch) {
        run_dedicated_server(launch);
        return;
    }
    run_client(launch);
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn run_dedicated_server(launch: LaunchMode) {
    use bevy::app::ScheduleRunnerPlugin;
    use bevy::diagnostic::DiagnosticsPlugin;
    use bevy::log::LogPlugin;
    use bevy::state::app::StatesPlugin;
    use core::time::Duration;

    App::new()
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))),
            TransformPlugin,
            bevy::input::InputPlugin,
            LogPlugin::default(),
            StatesPlugin,
            DiagnosticsPlugin,
        ))
        .add_plugins(GamePlugin { launch })
        .run();
}

fn run_client(launch: LaunchMode) {
    App::new()
        .insert_resource(ClearColor(Color::linear_rgb(0.4, 0.4, 0.4)))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Inventory Jam".to_string(),
                        canvas: Some("#bevy".to_owned()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_plugins(GamePlugin { launch })
        .add_systems(Startup, set_window_icon)
        .run();
}

#[cfg(not(target_arch = "wasm32"))]
fn set_window_icon(
    primary_window: Single<Entity, With<bevy::window::PrimaryWindow>>,
    _non_send_marker: bevy::ecs::system::NonSendMarker,
) -> Result {
    use bevy::winit::WINIT_WINDOWS;
    use std::io::Cursor;
    use winit::window::Icon;

    WINIT_WINDOWS.with_borrow(|windows| {
        let Some(primary) = windows.get_window(*primary_window) else {
            return Err(BevyError::from("No primary window!"));
        };
        let icon_buf = Cursor::new(include_bytes!("../assets/textures/bevy.png"));
        if let Ok(image) = image::load(icon_buf, image::ImageFormat::Png) {
            let image = image.into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            let icon = Icon::from_rgba(rgba, width, height).unwrap();
            primary.set_window_icon(Some(icon));
        };

        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn set_window_icon() {}
