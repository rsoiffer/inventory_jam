use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use bevy::prelude::*;

use super::state::PersistentState;
use super::transaction::PersistenceTransaction;

pub const PERSISTENCE_FLUSH_SECONDS: u64 = 5;

#[derive(Resource, Debug, Clone)]
pub struct PersistenceConfig {
    pub database_url: String,
    pub flush_interval: Duration,
}

impl PersistenceConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            database_url: read_database_url()?,
            flush_interval: Duration::from_secs(PERSISTENCE_FLUSH_SECONDS),
        })
    }
}

const DATABASE_SERVER_URL_PATH: &str = "secrets/database-server-url.txt";
const DATABASE_ADMIN_URL_PATH: &str = "secrets/database-admin-url.txt";

pub fn read_database_url() -> Result<String, String> {
    read_secret_url(DATABASE_SERVER_URL_PATH)
}

pub fn read_admin_database_url() -> Result<String, String> {
    read_secret_url(DATABASE_ADMIN_URL_PATH)
}

fn read_secret_url(path: &str) -> Result<String, String> {
    let contents =
        std::fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let url = contents.trim().to_string();
    if url.is_empty() {
        return Err(format!("{path} must not be empty"));
    }
    Ok(url)
}

#[derive(Resource, Default)]
pub struct PersistenceQueue {
    pub pending: VecDeque<PersistenceTransaction>,
    pub in_flight: VecDeque<PersistenceTransaction>,
    pub flush_boundary_hashes: HashMap<String, String>,
}

impl PersistenceQueue {
    pub fn enqueue(&mut self, transaction: PersistenceTransaction) {
        self.pending.push_back(transaction);
    }

    pub fn begin_flush(
        &mut self,
        connected_states: &HashMap<String, PersistentState>,
    ) -> (Vec<PersistenceTransaction>, HashMap<String, String>) {
        debug_assert!(self.in_flight.is_empty());
        self.in_flight = std::mem::take(&mut self.pending);
        self.flush_boundary_hashes = connected_states
            .iter()
            .map(|(username, state)| (username.clone(), state.hash_hex(username)))
            .collect();
        (
            self.in_flight.iter().cloned().collect(),
            self.flush_boundary_hashes.clone(),
        )
    }

    pub fn clear_in_flight(&mut self) {
        self.in_flight.clear();
        self.flush_boundary_hashes.clear();
    }

    pub fn retain_in_flight_for_retry(&mut self) {
        // Keep in_flight and boundary hashes so an uncertain flush can be retried.
    }

    pub fn abandon_in_flight(&mut self) {
        self.in_flight.clear();
        self.flush_boundary_hashes.clear();
    }

    pub fn pending_for_username<'a>(
        &'a self,
        username: &'a str,
    ) -> impl Iterator<Item = &'a PersistenceTransaction> {
        self.pending
            .iter()
            .filter(move |transaction| transaction.username == username)
    }
}
