use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use bevy::prelude::*;

use super::state::PersistentState;
use super::transaction::Transaction;

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

fn read_database_url() -> Result<String, String> {
    let contents = std::fs::read_to_string(DATABASE_SERVER_URL_PATH)
        .map_err(|error| format!("failed to read {DATABASE_SERVER_URL_PATH}: {error}"))?;
    let url = contents.trim().to_string();
    if url.is_empty() {
        return Err(format!("{DATABASE_SERVER_URL_PATH} must not be empty"));
    }
    Ok(url)
}

#[derive(Resource, Default)]
pub struct PendingTransactionQueue {
    pub pending: VecDeque<Transaction>,
    pub in_flight: VecDeque<Transaction>,
    pub flush_boundary_hashes: HashMap<String, String>,
}

impl PendingTransactionQueue {
    pub fn enqueue(&mut self, transaction: Transaction) {
        self.pending.push_back(transaction);
    }

    pub fn begin_flush(
        &mut self,
        connected_states: &HashMap<String, PersistentState>,
    ) -> (Vec<Transaction>, HashMap<String, String>) {
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

    pub fn abandon_in_flight(&mut self) {
        self.in_flight.clear();
        self.flush_boundary_hashes.clear();
    }

    pub fn pending_for_username<'a>(
        &'a self,
        username: &'a str,
    ) -> impl Iterator<Item = &'a Transaction> {
        self.pending
            .iter()
            .filter(move |transaction| transaction.touches_username(username))
    }
}
