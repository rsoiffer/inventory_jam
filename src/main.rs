//! Inventory Jam executable wiring for Bevy, Lightyear networking, and Edgegap discovery.

/// App construction and run-mode selection.
mod app;
#[cfg(feature = "client")]
/// Client gameplay systems.
mod client;
/// Low-level networking transport, connection setup, and Edgegap hosting.
mod networking;
/// Replicated protocol definitions.
mod protocol;
#[cfg(feature = "gui")]
/// Simple 2D debug renderer.
mod renderer;
#[cfg(feature = "server")]
/// Server gameplay systems.
mod server;

/// Shared gameplay logic.
mod shared;

use crate::app::{RunMode, build_app, run_mode};
#[cfg(feature = "client")]
use crate::client::ClientPlugin;
use crate::networking::{FIXED_TIMESTEP_HZ, spawn_connections};
#[cfg(feature = "server")]
use crate::server::ServerPlugin;
use crate::shared::SharedPlugin;
use core::time::Duration;

/// Process entrypoint.
fn main() {
    let mode = run_mode();
    let mut app = build_app(mode, Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ));
    app.add_plugins(SharedPlugin);
    spawn_connections(&mut app, mode);

    match mode {
        #[cfg(feature = "client")]
        RunMode::Client { .. } => {
            app.add_plugins(ClientPlugin);
        }
        #[cfg(feature = "server")]
        RunMode::Server => {
            app.add_plugins(ServerPlugin);
        }
    }

    #[cfg(feature = "gui")]
    app.add_plugins(renderer::RendererPlugin);

    app.run();
}
