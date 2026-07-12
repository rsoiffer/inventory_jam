//! Run with:
//! - `cargo run -- server`
//! - `cargo run -- client -c 1`

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[cfg(feature = "client")]
mod client;
mod enemy;
mod fragment;
mod networking;
mod projectile;
mod protocol;
#[cfg(feature = "gui")]
mod renderer;
#[cfg(feature = "server")]
mod server;
mod shared;

#[cfg(feature = "client")]
use crate::client::ExampleClientPlugin;
use crate::networking::{spawn_connections, RunMode, FIXED_TIMESTEP_HZ};
#[cfg(feature = "server")]
use crate::server::ExampleServerPlugin;
use crate::shared::SharedPlugin;
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use core::time::Duration;

#[cfg(all(not(feature = "gui"), feature = "client"))]
const HEADLESS_CLIENT_LOOP_HZ: f64 = 60.0;

fn main() {
    let mode = run_mode();
    let mut app = build_app(mode, Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ));
    app.add_plugins(SharedPlugin);
    spawn_connections(&mut app, mode);

    match mode {
        #[cfg(feature = "client")]
        RunMode::Client { .. } => {
            app.add_plugins((ExampleClientPlugin, fragment::FragmentClientPlugin));
        }
        #[cfg(feature = "server")]
        RunMode::Server => {
            app.add_plugins((ExampleServerPlugin, fragment::FragmentServerPlugin));
        }
    }

    #[cfg(feature = "gui")]
    app.add_plugins((
        renderer::ExampleRendererPlugin,
        fragment::FragmentRenderPlugin,
    ));

    app.run();
}

fn run_mode() -> RunMode {
    #[cfg(target_family = "wasm")]
    {
        RunMode::Client {
            client_id: rand::random(),
            use_local_server: false,
        }
    }
    #[cfg(not(target_family = "wasm"))]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let use_local_server = args.iter().any(|arg| arg == "--dev");
        match args.first().map(String::as_str) {
            #[cfg(feature = "server")]
            Some("server") => RunMode::Server,
            #[cfg(feature = "client")]
            Some("client") => RunMode::Client {
                client_id: parse_client_id(args.iter().skip(1).cloned()),
                use_local_server,
            },
            #[cfg(all(feature = "server", not(feature = "client")))]
            _ => RunMode::Server,
            #[cfg(feature = "client")]
            _ => RunMode::Client {
                client_id: 1,
                use_local_server,
            },
        }
    }
}

#[cfg(all(not(target_family = "wasm"), feature = "client"))]
fn parse_client_id(args: impl Iterator<Item = String>) -> u64 {
    let mut client_id = 1;
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "-c" {
            if let Some(id) = args.next().and_then(|value| value.parse().ok()) {
                client_id = id;
            }
        } else if let Ok(id) = arg.parse() {
            client_id = id;
        }
    }
    client_id
}

fn build_app(mode: RunMode, tick_duration: Duration) -> App {
    let mut app = create_app(mode);
    #[cfg(feature = "server")]
    app.add_observer(crate::networking::apply_server_link_conditioner);
    match mode {
        #[cfg(feature = "client")]
        RunMode::Client { .. } => {
            app.add_plugins(lightyear::prelude::client::ClientPlugins { tick_duration });
        }
        #[cfg(feature = "server")]
        RunMode::Server => {
            app.add_plugins(lightyear::prelude::server::ServerPlugins { tick_duration });
        }
    }
    app
}

fn create_app(mode: RunMode) -> App {
    #[cfg(feature = "gui")]
    {
        let _ = mode;
        new_gui_app()
    }
    #[cfg(not(feature = "gui"))]
    {
        let loop_wait = match mode {
            #[cfg(feature = "client")]
            RunMode::Client { .. } => Some(Duration::from_secs_f64(1.0 / HEADLESS_CLIENT_LOOP_HZ)),
            #[cfg(feature = "server")]
            RunMode::Server => None,
        };
        new_headless_app(loop_wait)
    }
}

#[cfg(feature = "gui")]
fn new_gui_app() -> App {
    use bevy::window::PresentMode;
    use bevy::winit::WinitSettings;

    let mut app = App::new();
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
                    title: env!("CARGO_PKG_NAME").to_string(),
                    resolution: (1024, 768).into(),
                    present_mode: PresentMode::AutoVsync,
                    prevent_default_event_handling: true,
                    ..default()
                }),
                ..default()
            }),
    );
    app.insert_resource(WinitSettings::continuous());
    #[cfg(feature = "dev")]
    app.add_plugins((
        bevy_inspector_egui::bevy_egui::EguiPlugin::default(),
        bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
    ));
    app
}

#[cfg(not(feature = "gui"))]
fn new_headless_app(loop_wait: Option<Duration>) -> App {
    let mut app = App::new();
    let minimal_plugins = match loop_wait {
        Some(wait) => MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(wait)),
        None => MinimalPlugins.build(),
    };
    app.add_plugins((
        minimal_plugins,
        TransformPlugin,
        bevy::input::InputPlugin,
        log_plugin(),
        bevy::state::app::StatesPlugin,
        bevy::diagnostic::DiagnosticsPlugin,
    ));
    app
}

fn log_plugin() -> LogPlugin {
    LogPlugin {
        level: Level::INFO,
        filter: "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn,naga=warn".to_string(),
        ..default()
    }
}
