//! Inventory Jam executable wiring for Bevy, Lightyear networking, and Edgegap discovery.
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

/// App construction and run-mode selection.
mod app;
#[cfg(feature = "client")]
/// Client gameplay systems.
mod client;
/// Edgegap runtime environment helpers.
mod edgegap;
/// Low-level Lightyear transport components and constants.
mod net;
/// Client/server connection setup.
mod networking;
/// Replicated protocol definitions.
mod protocol;
#[cfg(feature = "gui")]
/// Simple 2D debug renderer.
mod renderer;
#[cfg(feature = "server")]
/// Server gameplay systems.
mod server;
#[cfg(feature = "client")]
/// Edgegap server discovery for remote clients.
mod server_discovery;

/// Shared gameplay logic.
mod shared;

use crate::app::{RunMode, build_app, run_mode};
#[cfg(feature = "client")]
use crate::client::ExampleClientPlugin;
use crate::net::FIXED_TIMESTEP_HZ;
use crate::networking::spawn_connections;
#[cfg(feature = "server")]
use crate::server::ExampleServerPlugin;
use crate::shared::SharedPlugin;
use bevy::prelude::*;
use core::time::Duration;

/// Process entrypoint.
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        #[cfg(not(target_arch = "wasm32"))]
        std::process::exit(1);
    }
}

/// Builds and runs the Bevy app for the selected mode.
fn run() -> Result<(), String> {
    let mode = run_mode();
    let mut app = build_app(mode, Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ));
    app.add_plugins(SharedPlugin);
    spawn_connections(&mut app, mode)?;

    match mode {
        #[cfg(feature = "client")]
        RunMode::Client { .. } => {
            app.add_plugins(ExampleClientPlugin);
        }
        #[cfg(feature = "server")]
        RunMode::Server => {
            app.add_plugins(ExampleServerPlugin);
        }
    }

    #[cfg(feature = "gui")]
    app.add_plugins(renderer::ExampleRendererPlugin);

    app.run();
    Ok(())
}
