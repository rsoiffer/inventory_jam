mod api;
mod state;

#[cfg(feature = "server")]
mod queue;
#[cfg(feature = "server")]
mod systems;
#[cfg(feature = "server")]
mod transaction;
#[cfg(feature = "server")]
mod worker;

use bevy::prelude::*;
use lightyear::prelude::*;

#[cfg(feature = "server")]
use crate::app::AppState;
#[cfg(feature = "server")]
use crate::shared::FixedGameplaySet;

#[cfg(feature = "server")]
pub use api::{AddFragments, PersistenceReady};
pub use state::CachedPersistentState;

#[cfg(feature = "server")]
use queue::{PersistenceConfig, PersistenceQueue};

#[cfg(feature = "server")]
use systems::{
    apply_add_fragments_messages, apply_username_messages, flush_persistence_queue,
    poll_persistence_responses,
};
#[cfg(feature = "server")]
use worker::{PersistenceFlushClock, spawn_worker};

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PersistenceProtocolPlugin);

        #[cfg(feature = "server")]
        app.add_plugins(PersistenceServerPlugin);
    }
}

/// Registers replicated persistence components on every peer.
struct PersistenceProtocolPlugin;

impl Plugin for PersistenceProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<CachedPersistentState>().replicate();
    }
}

#[cfg(feature = "server")]
struct PersistenceServerPlugin;

#[cfg(feature = "server")]
impl Plugin for PersistenceServerPlugin {
    fn build(&self, app: &mut App) {
        let config = PersistenceConfig::from_env().unwrap_or_else(|error| {
            panic!("persistence configuration error: {error}");
        });
        let channels = spawn_worker(config.database_url.clone()).unwrap_or_else(|error| {
            panic!("failed to start persistence worker: {error:#}");
        });

        app.add_message::<AddFragments>()
            .insert_resource(config.clone())
            .insert_resource(channels)
            .insert_resource(PersistenceQueue::default())
            .insert_resource(PersistenceFlushClock::new(config.flush_interval))
            .add_systems(
                FixedUpdate,
                apply_add_fragments_messages
                    .in_set(FixedGameplaySet::Persistence)
                    .run_if(in_state(AppState::Hosting)),
            )
            .add_systems(
                Update,
                (
                    apply_username_messages,
                    poll_persistence_responses,
                    flush_persistence_queue,
                )
                    .chain()
                    .run_if(in_state(AppState::Hosting)),
            );
    }
}
