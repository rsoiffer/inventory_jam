mod app;
mod enemy;
mod fragment;
mod networking;
mod persistence;
mod player;
mod projectile;
mod protocol;
#[cfg(feature = "server")]
mod server;
mod settings;
mod shared;
mod ui;
mod weapon;
mod world;

use crate::settings::{GameSettings, GameSettingsLoadError};
use app::*;
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
#[cfg(feature = "gui")]
use bevy::window::PresentMode;
#[cfg(feature = "gui")]
use bevy::winit::WinitSettings;
use enemy::EnemyPlugin;
use fragment::FragmentPlugin;
use persistence::PersistencePlugin;
use player::PlayerPlugin;
use projectile::ProjectilePlugin;
#[cfg(feature = "server")]
use server::ExampleServerPlugin;
use shared::SharedPlugin;
use std::{process::ExitCode, time::Duration};
use weapon::WeaponPlugin;
use world::WorldPlugin;

use crate::ui::UiPlugin;

const TICK_DURATION: Duration =
    Duration::from_nanos((1_000_000_000.0 / networking::FIXED_TIMESTEP_HZ) as u64);

#[cfg(not(feature = "gui"))]
const HEADLESS_CLIENT_LOOP_HZ: f64 = 60.0;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Application startup failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), GameSettingsLoadError> {
    let settings = GameSettings::load("assets/game_settings.json")?;

    let config = config_from_env();
    let mut app = base_app();
    app.insert_resource(settings)
        .insert_resource(config.clone())
        .insert_resource(LocalUsername(config.username.clone()));

    #[cfg(feature = "server")]
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: TICK_DURATION,
    });

    #[cfg(feature = "client")]
    app.add_plugins(lightyear::prelude::client::ClientPlugins {
        tick_duration: TICK_DURATION,
    });

    #[cfg(feature = "client")]
    app.init_state::<ClientState>();

    #[cfg(feature = "server")]
    app.init_state::<ServerState>();

    app.add_plugins((
        WorldPlugin,
        UiPlugin,
        LaunchPlugin,
        SharedPlugin,
        PlayerPlugin,
        EnemyPlugin,
        ProjectilePlugin,
        WeaponPlugin,
        FragmentPlugin,
        PersistencePlugin,
    ));

    networking::configure_networking(&mut app);

    #[cfg(feature = "server")]
    app.add_observer(networking::apply_server_link_conditioner)
        .add_plugins(ExampleServerPlugin);

    app.run();
    Ok(())
}

fn base_app() -> App {
    let mut app = App::new();

    #[cfg(feature = "gui")]
    {
        use crate::shared::GAME_NAME;

        app.add_plugins(
            DefaultPlugins
                .build()
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
                .set(bevy::image::ImagePlugin::default_nearest())
                .set(log_plugin())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: GAME_NAME.to_owned(),
                        resolution: (1024, 768).into(),
                        present_mode: PresentMode::AutoVsync,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(WinitSettings::continuous());
        #[cfg(feature = "dev")]
        app.add_plugins((
            bevy_inspector_egui::bevy_egui::EguiPlugin::default(),
            bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
        ));
        app
    }

    #[cfg(not(feature = "gui"))]
    {
        app.add_plugins((
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                Duration::from_secs_f64(1.0 / HEADLESS_CLIENT_LOOP_HZ),
            )),
            TransformPlugin,
            bevy::input::InputPlugin,
            log_plugin(),
            bevy::state::app::StatesPlugin,
            bevy::diagnostic::DiagnosticsPlugin,
        ));
        app
    }
}

fn log_plugin() -> LogPlugin {
    LogPlugin {
        level: Level::INFO,
        filter: "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn,naga=warn".to_string(),
        ..default()
    }
}
