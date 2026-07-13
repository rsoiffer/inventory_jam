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
pub use api::PersistenceReady;
pub use state::CachedPersistentState;
#[cfg(feature = "server")]
pub use transaction::Transaction;

#[cfg(feature = "server")]
use queue::{PendingTransactionQueue, read_database_url};

#[cfg(feature = "server")]
use systems::{
    apply_persistence_transactions, apply_username_messages, flush_persistence_queue,
    poll_persistence_responses,
};
#[cfg(feature = "server")]
use worker::spawn_worker;

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
        let database_url = read_database_url().unwrap_or_else(|error| {
            panic!("persistence configuration error: {error}");
        });
        let channels = spawn_worker(database_url).unwrap_or_else(|error| {
            panic!("failed to start persistence worker: {error:#}");
        });

        app.add_message::<Transaction>()
            .insert_resource(channels)
            .insert_resource(PendingTransactionQueue::default())
            .add_systems(
                FixedUpdate,
                apply_persistence_transactions
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
