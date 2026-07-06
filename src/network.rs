use crate::config::FIXED_TIMESTEP_HZ;
use crate::launch::LaunchMode;
use crate::network_log::NetworkLoggingPlugin;
use crate::shared::SharedPlugin;
use bevy::prelude::*;
use core::time::Duration;

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use crate::client::maybe_spawn_host_client;
#[cfg(feature = "client")]
use crate::client::{connect_pending_clients, maybe_spawn_join_client};
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use crate::server::{maybe_spawn_server, spawn_dedicated_server, start_pending_servers};

pub struct NetworkPlugin {
    pub launch: LaunchMode,
}

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        match &self.launch {
            #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
            LaunchMode::DedicatedServer { bind_addr } => {
                info!("[net] Dedicated server mode — binding to {bind_addr}");
                app.insert_resource(crate::server::ServerBindAddr(*bind_addr));
                app.add_plugins(lightyear::prelude::server::ServerPlugins {
                    tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
                });
                app.add_plugins(SharedPlugin);
                app.add_plugins(crate::server::ExampleServerPlugin);
                app.add_plugins(NetworkLoggingPlugin);
                app.add_systems(
                    Startup,
                    (spawn_dedicated_server, start_pending_servers).chain(),
                );
            }
            LaunchMode::Client { session } => {
                info!(
                    "[net] Session configured — {:?} client_id={} server={}",
                    session.mode, session.client_id, session.server_addr
                );
                app.register_type::<crate::config::NetworkSession>()
                    .register_type::<crate::config::SessionMode>()
                    .insert_resource(session.clone());

                #[cfg(feature = "client")]
                {
                    app.add_plugins(lightyear::prelude::client::ClientPlugins {
                        tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
                    });
                }

                #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
                if session.mode == crate::config::SessionMode::Host {
                    app.add_plugins(lightyear::prelude::server::ServerPlugins {
                        tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
                    });
                }

                // Protocol must be registered after Lightyear client/server plugins.
                app.add_plugins(SharedPlugin);

                #[cfg(feature = "client")]
                {
                    app.add_plugins(crate::client::ExampleClientPlugin);
                }

                #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
                if session.mode == crate::config::SessionMode::Host {
                    app.add_plugins(crate::server::ExampleServerPlugin);
                }

                register_playing_session(app);
                app.add_plugins(NetworkLoggingPlugin);
            }
        }
    }
}

fn register_playing_session(app: &mut App) {
    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    app.add_systems(
        OnEnter(crate::GameState::Playing),
        log_playing_session.before(maybe_spawn_server),
    )
    .add_systems(
        OnEnter(crate::GameState::Playing),
        (
            maybe_spawn_server,
            start_pending_servers,
            maybe_spawn_host_client,
            maybe_spawn_join_client,
            connect_pending_clients,
        )
            .chain(),
    );

    #[cfg(not(all(feature = "server", not(target_arch = "wasm32"))))]
    app.add_systems(
        OnEnter(crate::GameState::Playing),
        log_playing_session.before(maybe_spawn_join_client),
    )
    .add_systems(
        OnEnter(crate::GameState::Playing),
        (maybe_spawn_join_client, connect_pending_clients).chain(),
    );
}

fn log_playing_session(session: Option<Res<crate::config::NetworkSession>>) {
    let Some(session) = session else {
        warn!("[net] Entered playing state without a network session");
        return;
    };
    info!(
        "[net] Entering multiplayer — {:?} client_id={} server={}",
        session.mode, session.client_id, session.server_addr
    );
}
