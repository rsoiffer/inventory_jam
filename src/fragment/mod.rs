use crate::app::{AppState, game_is_active};
use bevy::prelude::*;

pub mod api;

#[cfg(feature = "client")]
pub(crate) mod client;

pub(crate) mod protocol;

#[cfg(feature = "gui")]
pub(crate) mod render;

#[cfg(feature = "server")]
pub(crate) mod server;

pub mod shared;

pub use api::SpawnFragmentPool;

/// Installs network component registration on every peer.
pub struct FragmentProtocolPlugin;

impl Plugin for FragmentProtocolPlugin {
    fn build(&self, app: &mut App) {
        protocol::register(app);
    }
}

/// Installs only client-side fragment presentation behavior.
#[cfg(feature = "client")]
pub struct FragmentClientPlugin;

#[cfg(feature = "client")]
impl Plugin for FragmentClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(client::initialize_fragment);
        app.add_systems(
            FixedUpdate,
            client::simulate_client_fragments.run_if(in_state(AppState::Playing)),
        );
    }
}

/// Installs only server-authoritative fragment behavior.
#[cfg(feature = "server")]
pub struct FragmentServerPlugin;

#[cfg(feature = "server")]
impl Plugin for FragmentServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnFragmentPool>();

        app.add_systems(
            FixedUpdate,
            (
                server::ensure_player_fragment_wallets,
                server::request_debug_fragment_pool,
                server::spawn_requested_fragment_pools,
                server::simulate_server_fragments,
            )
                .chain()
                .run_if(in_state(AppState::Hosting)),
        );
    }
}

/// Installs fragment rendering without exposing rendering internals elsewhere.
#[cfg(feature = "gui")]
pub struct FragmentRenderPlugin;

#[cfg(feature = "gui")]
impl Plugin for FragmentRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Playing), render::setup_fragment_balance);
        app.add_systems(
            OnExit(AppState::Playing),
            |mut commands: Commands, texts: Query<Entity, With<render::FragmentBalanceText>>| {
                for entity in &texts {
                    commands.entity(entity).despawn();
                }
            },
        );

        app.add_systems(Update, render::draw_fragments.run_if(game_is_active));
        app.add_systems(
            Update,
            render::update_fragment_balance.run_if(in_state(AppState::Playing)),
        );
    }
}
