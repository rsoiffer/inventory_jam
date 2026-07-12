use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
#[cfg(feature = "gui")]
use bevy::window::PresentMode;
#[cfg(feature = "gui")]
use bevy::winit::WinitSettings;
use inventory_jam::app::*;
#[cfg(feature = "client")]
use inventory_jam::client::ExampleClientPlugin;
#[cfg(feature = "gui")]
use inventory_jam::renderer;
#[cfg(feature = "server")]
use inventory_jam::server::ExampleServerPlugin;
use inventory_jam::shared::SharedPlugin;
#[cfg(feature = "gui")]
use inventory_jam::ui::MenuPlugin;
use inventory_jam::{fragment, networking, weapon};
use std::time::Duration;

const TICK_DURATION: Duration =
    Duration::from_nanos((1_000_000_000.0 / networking::FIXED_TIMESTEP_HZ) as u64);

#[cfg(all(not(feature = "gui"), feature = "client"))]
const HEADLESS_CLIENT_LOOP_HZ: f64 = 60.0;

fn main() {
    let config = config_from_env();
    let mut app = base_app(&config);
    app.insert_resource(config.clone())
        .insert_resource(LocalUsername(config.username.clone()));

    #[cfg(feature = "server")]
    if config.mode.starts_server() {
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: TICK_DURATION,
        });
    }

    #[cfg(feature = "client")]
    if config.mode.starts_client() {
        app.add_plugins(lightyear::prelude::client::ClientPlugins {
            tick_duration: TICK_DURATION,
        });
    }

    app.add_plugins((
        SharedPlugin,
        fragment::FragmentProtocolPlugin,
        weapon::WeaponProtocolPlugin,
    ));
    networking::configure_networking(&mut app, config.mode);

    #[cfg(feature = "server")]
    if config.mode.starts_server() {
        app.add_observer(networking::apply_server_link_conditioner);
        app.add_plugins((ExampleServerPlugin, fragment::FragmentServerPlugin));
    }

    #[cfg(feature = "client")]
    if config.mode.starts_client() {
        app.add_plugins((ExampleClientPlugin, fragment::FragmentClientPlugin));
    }

    #[cfg(feature = "gui")]
    app.insert_state(match config.mode {
        LaunchMode::DedicatedServer | LaunchMode::HostLocal => AppState::Hosting,
        LaunchMode::JoinEdgegap | LaunchMode::JoinLocal => AppState::Connecting,
        LaunchMode::Menu => AppState::MainMenu,
    })
    .add_plugins((
        renderer::ExampleRendererPlugin,
        fragment::FragmentRenderPlugin,
        MenuPlugin,
    ));

    app.run();
}

fn base_app(_config: &LaunchConfig) -> App {
    let mut app = App::new();

    #[cfg(feature = "gui")]
    {
        app.add_plugins(
            DefaultPlugins
                .build()
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
                .set(log_plugin())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: env!("CARGO_PKG_NAME").into(),
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
        let plugins = match _config.mode {
            #[cfg(feature = "client")]
            LaunchMode::JoinEdgegap | LaunchMode::JoinLocal => {
                MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                    Duration::from_secs_f64(1.0 / HEADLESS_CLIENT_LOOP_HZ),
                ))
            }
            _ => MinimalPlugins.build(),
        };
        app.add_plugins((
            plugins,
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
        filter: "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn,naga=warn".into(),
        ..default()
    }
}
