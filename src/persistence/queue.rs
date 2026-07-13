use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use uuid::Uuid;

use super::transaction::Transaction;

pub const FLUSH_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Resource, Debug, Clone)]
pub struct PersistenceConfig {
    pub database_url: String,
}

impl PersistenceConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            database_url: read_database_url()?,
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

    #[cfg(test)]
    fn due_now() -> Self {
        Self {
            pending: VecDeque::new(),
            next_flush_at: Instant::now()
                .checked_sub(Duration::from_secs(1))
                .unwrap_or_else(Instant::now),
            is_flushing: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acknowledge_keeps_transactions_enqueued_during_flush() {
        let mut queue = PendingTransactionQueue::due_now();
        let first = Transaction::add_fragments("alice", 1);
        let first_id = first.id;
        queue.enqueue(first);

        let snapshot = queue.begin_flush_if_due().expect("flush should start");
        assert_eq!(snapshot.len(), 1);
        assert!(queue.is_flushing());

        let second = Transaction::add_fragments("alice", 2);
        let second_id = second.id;
        queue.enqueue(second);

        queue.acknowledge(&[first_id]);
        let remaining: Vec<_> = queue
            .pending_for_username("alice")
            .map(|tx| tx.id)
            .collect();
        assert_eq!(remaining, vec![second_id]);

        queue.finish_flush();
        assert!(!queue.is_flushing());
        assert!(queue.pending_for_username("alice").next().is_some());
    }

    #[test]
    fn begin_flush_is_blocked_while_flushing() {
        let mut queue = PendingTransactionQueue::due_now();
        queue.enqueue(Transaction::add_fragments("alice", 1));
        assert!(queue.begin_flush_if_due().is_some());
        assert!(queue.begin_flush_if_due().is_none());
    }
}
