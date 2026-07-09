use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use core::time::Duration;

/// Update frequency for headless clients so they keep processing network events.
const HEADLESS_CLIENT_LOOP_HZ: f64 = 60.0;

/// Startup mode selected from the target platform and command-line arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    #[cfg(feature = "client")]
    /// Runs a client using the provided Lightyear client id.
    Client {
        /// Stable id used by netcode authentication.
        client_id: u64,
    },
    #[cfg(feature = "server")]
    /// Runs the authoritative server.
    Server,
}

/// Determines whether this process should run as a client or server.
pub fn run_mode() -> RunMode {
    #[cfg(target_family = "wasm")]
    {
        RunMode::Client {
            client_id: rand::random(),
        }
    }
    #[cfg(not(target_family = "wasm"))]
    {
        let mut args = std::env::args().skip(1);
        match args.next().as_deref() {
            #[cfg(feature = "server")]
            Some("server") => RunMode::Server,
            #[cfg(feature = "client")]
            Some("client") => RunMode::Client {
                client_id: parse_client_id(&mut args),
            },
            #[cfg(all(feature = "server", not(feature = "client")))]
            _ => RunMode::Server,
            #[cfg(feature = "client")]
            _ => RunMode::Client { client_id: 1 },
        }
    }
}

#[cfg(not(target_family = "wasm"))]
/// Parses a client id from `client -c <id>`, `client <id>`, or defaults to one.
fn parse_client_id(args: &mut impl Iterator<Item = String>) -> u64 {
    let mut client_id = 1;
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

/// Builds a Bevy app with the Lightyear plugin set for the selected run mode.
pub fn build_app(mode: RunMode, tick_duration: Duration) -> App {
    let mut app = create_app(mode);
    #[cfg(feature = "server")]
    app.add_observer(crate::net::server::apply_server_link_conditioner);
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

/// Creates either the windowed or headless base app.
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
/// Creates a windowed Bevy app with rendering and optional development tooling.
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
    app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
    app
}

#[cfg(not(feature = "gui"))]
/// Creates a minimal app for server or headless client execution.
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

/// Builds the shared log configuration used by both GUI and headless apps.
fn log_plugin() -> LogPlugin {
    LogPlugin {
        level: Level::INFO,
        filter: "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn,naga=warn".to_string(),
        ..default()
    }
}
