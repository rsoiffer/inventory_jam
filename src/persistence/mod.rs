mod model;

#[cfg(feature = "server")]
mod systems;
#[cfg(feature = "server")]
mod worker;

use bevy::prelude::*;
use lightyear::prelude::*;

#[cfg(feature = "server")]
use crate::app::AppState;
#[cfg(feature = "server")]
use crate::shared::FixedGameplaySet;

pub use model::CachedPersistentState;
#[cfg(feature = "server")]
pub use model::{PersistenceReady, Transaction};

#[cfg(feature = "server")]
use systems::{
    PendingTransactionQueue, apply_persistence_transactions, apply_username_messages,
    flush_persistence_queue, poll_persistence_responses,
};
#[cfg(feature = "server")]
use worker::spawn_worker;

#[cfg(feature = "server")]
const DATABASE_SERVER_URL_PATH: &str = "secrets/database-server-url.txt";

#[cfg(feature = "server")]
fn read_database_url() -> Result<String, String> {
    let contents = std::fs::read_to_string(DATABASE_SERVER_URL_PATH)
        .map_err(|error| format!("failed to read {DATABASE_SERVER_URL_PATH}: {error}"))?;
    let url = contents.trim().to_string();
    if url.is_empty() {
        return Err(format!("{DATABASE_SERVER_URL_PATH} must not be empty"));
    }
    Ok(url)
}

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
            eprintln!("persistence configuration error: {error}");
            std::process::exit(1);
        });
        let channels = spawn_worker(database_url).unwrap_or_else(|error| {
            eprintln!("failed to start persistence worker: {error:#}");
            std::process::exit(1);
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
