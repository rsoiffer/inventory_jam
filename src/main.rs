//! Run with:
//! - `cargo run -- server`
//! - `cargo run -- client -c 1`

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[cfg(feature = "client")]
use crate::client::ExampleClientPlugin;

#[cfg(feature = "server")]
use crate::server::ExampleServerPlugin;

use crate::shared::SharedPlugin;
use bevy::prelude::*;
use core::time::Duration;
use lightyear_examples_common::cli::Mode;
use lightyear_examples_common::shared::FIXED_TIMESTEP_HZ;

#[cfg(feature = "client")]
mod client;

mod fragment;
mod projectile;
mod protocol;

#[cfg(feature = "gui")]
mod renderer;

#[cfg(feature = "server")]
mod server;

mod shared;

fn main() {
    // Do not use Cli::default().
    // That ignores `server` and `client -c 1` command-line arguments.
    let cli = lightyear_examples_common::cli::cli();

    let mut app = cli.build_app(Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ), true);

    // These are installed on every runtime.
    // Client and server must register the same network protocol.
    app.add_plugins((SharedPlugin, fragment::FragmentProtocolPlugin));

    // Install runtime behavior based on the selected mode.
    match cli.mode {
        #[cfg(feature = "client")]
        Some(Mode::Client { .. }) => {
            app.add_plugins((ExampleClientPlugin, fragment::FragmentClientPlugin));
        }

        #[cfg(feature = "server")]
        Some(Mode::Server) => {
            app.add_plugins((ExampleServerPlugin, fragment::FragmentServerPlugin));
        }

        #[cfg(all(feature = "client", feature = "server"))]
        Some(Mode::HostClient { .. }) => {
            app.add_plugins((
                ExampleClientPlugin,
                ExampleServerPlugin,
                fragment::FragmentClientPlugin,
                fragment::FragmentServerPlugin,
            ));
        }

        _ => {}
    }

    #[cfg(feature = "gui")]
    app.add_plugins((
        renderer::ExampleRendererPlugin,
        fragment::FragmentRenderPlugin,
    ));

    // Spawn connection entities after feature plugins and protocol
    // registrations have been installed.
    cli.spawn_connections(&mut app);

    app.run();
}
