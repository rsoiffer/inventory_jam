use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use uuid::Uuid;

use super::transaction::Transaction;

pub const FLUSH_INTERVAL: Duration = Duration::from_secs(5);

const DATABASE_SERVER_URL_PATH: &str = "secrets/database-server-url.txt";

pub fn read_database_url() -> Result<String, String> {
    let contents = std::fs::read_to_string(DATABASE_SERVER_URL_PATH)
        .map_err(|error| format!("failed to read {DATABASE_SERVER_URL_PATH}: {error}"))?;
    let url = contents.trim().to_string();
    if url.is_empty() {
        return Err(format!("{DATABASE_SERVER_URL_PATH} must not be empty"));
    }
    Ok(url)
}

#[derive(Resource)]
pub struct PendingTransactionQueue {
    pending: VecDeque<Transaction>,
    next_flush_at: Instant,
    is_flushing: bool,
}

impl Default for PendingTransactionQueue {
    fn default() -> Self {
        Self {
            pending: VecDeque::new(),
            next_flush_at: Instant::now() + FLUSH_INTERVAL,
            is_flushing: false,
        }
    }
}

impl PendingTransactionQueue {
    pub fn enqueue(&mut self, transaction: Transaction) {
        self.pending.push_back(transaction);
    }

    pub fn is_flushing(&self) -> bool {
        self.is_flushing
    }

    /// If a flush is due and none is in progress, mark flushing and return a
    /// snapshot of the current pending batch. Transactions stay in `pending`
    /// until acknowledged after a successful flush.
    pub fn begin_flush_if_due(&mut self) -> Option<Vec<Transaction>> {
        if self.is_flushing || Instant::now() < self.next_flush_at {
            return None;
        }
        self.is_flushing = true;
        Some(self.pending.iter().cloned().collect())
    }

    pub fn acknowledge(&mut self, transaction_ids: &[Uuid]) {
        let ids: HashSet<Uuid> = transaction_ids.iter().copied().collect();
        self.pending
            .retain(|transaction| !ids.contains(&transaction.id));
    }

    pub fn pending_for_username<'a>(
        &'a self,
        username: &'a str,
    ) -> impl Iterator<Item = &'a Transaction> {
        self.pending
            .iter()
            .filter(move |transaction| transaction.touches_username(username))
    }

    pub fn finish_flush(&mut self) {
        self.is_flushing = false;
        self.next_flush_at = Instant::now() + FLUSH_INTERVAL;
    }
}
