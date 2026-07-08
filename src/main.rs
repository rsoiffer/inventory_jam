//! This minimal example showcases how to setup the lightyear plugins.
//!
//! Run with
//! - `cargo run -- server`
//! - `cargo run -- client`
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

mod automation;
#[cfg(feature = "client")]
mod client;
mod protocol;
mod renderer;
#[cfg(feature = "server")]
mod server;
mod shared;

#[cfg(feature = "client")]
use crate::automation::ClientStartupConfig;
#[cfg(feature = "server")]
use crate::automation::ServerStartupConfig;
use crate::shared::{FIXED_TIMESTEP_HZ, SharedPlugin};
use bevy::prelude::*;
use clap::{Parser, Subcommand, ValueEnum};
use core::time::Duration;
#[cfg(feature = "client")]
use lightyear::prelude::client::ClientPlugins;
#[cfg(feature = "server")]
use lightyear::prelude::server::ServerPlugins;

/// CLI options to create an [`App`]
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Mode,
}

#[derive(Subcommand, Debug)]
pub enum Mode {
    #[cfg(feature = "client")]
    Client,
    #[cfg(feature = "server")]
    Server,
    #[cfg(all(feature = "client", feature = "server"))]
    HostClient {
        #[arg(short, long, default_value_t = 0)]
        client_id: u64,
    },
}

fn main() {
    let cli = cli();
    let mut app = automation::build_base_app();

    match cli.mode {
        #[cfg(feature = "client")]
        Mode::Client => {
            // add lightyear plugins
            app.add_plugins(ClientPlugins {
                tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
            });
            // NOTE: the ProtocolPlugin must be added AFTER the Client/Server plugins,
            app.add_plugins(SharedPlugin);
            app.insert_resource(ClientStartupConfig {
                client_id: 0,
                host_server: None,
            });
            // add client-specific plugins
            app.add_plugins(client::ExampleClientPlugin);
            automation::spawn_client(&mut app, 0);
        }
        #[cfg(feature = "server")]
        Mode::Server => {
            app.add_plugins(ServerPlugins {
                tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
            });
            // NOTE: the ProtocolPlugin must be added AFTER the Client/Server plugins
            app.add_plugins(SharedPlugin);
            app.insert_resource(ServerStartupConfig::default());
            app.add_plugins(server::ExampleServerPlugin);
            automation::spawn_server(&mut app);
        }
        #[cfg(all(feature = "client", feature = "server"))]
        Mode::HostClient { client_id } => {
            app.add_plugins(ClientPlugins {
                tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
            });
            app.add_plugins(ServerPlugins {
                tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
            });
            app.add_plugins(SharedPlugin);
            app.add_plugins(server::ExampleServerPlugin);
            app.insert_resource(ServerStartupConfig { auto_spawn: false });
            // NOTE: it's important to start the server before connecting the host-client.
            let server = automation::spawn_server(&mut app);
            app.insert_resource(ClientStartupConfig {
                client_id,
                host_server: Some(server),
            });
            app.add_plugins(client::ExampleClientPlugin);
            automation::spawn_host_client(&mut app, server);
        }
    }

    #[cfg(feature = "gui")]
    app.add_plugins(renderer::ExampleRendererPlugin);

    app.run();
}

fn cli() -> Cli {
    cfg_if::cfg_if! {
        if #[cfg(target_family = "wasm")] {
            Cli { mode: Mode::Client }
        } else {
            Cli::parse()
        }
    }
}
