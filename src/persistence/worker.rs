use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use bevy::prelude::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use super::state::PersistentState;
use super::transaction::Transaction;

pub enum PersistenceRequest {
    LoadPlayer {
        username: String,
    },
    Flush {
        flush_id: u64,
        transactions: Vec<Transaction>,
        usernames: Vec<String>,
    },
}

pub enum PersistenceResponse {
    LoadPlayer {
        username: String,
        result: Result<PersistentState, String>,
    },
    Flush {
        flush_id: u64,
        result: Result<FlushResult, String>,
    },
}

#[derive(Debug, Clone)]
pub struct FlushResult {
    pub hashes: HashMap<String, String>,
}

#[derive(Resource)]
pub struct PersistenceChannels {
    pub request_tx: std::sync::mpsc::Sender<PersistenceRequest>,
    pub response_rx: std::sync::Mutex<std::sync::mpsc::Receiver<PersistenceResponse>>,
}

pub fn spawn_worker(database_url: String) -> Result<PersistenceChannels> {
    let (request_tx, request_rx) = std::sync::mpsc::channel();
    let (response_tx, response_rx) = std::sync::mpsc::channel();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<()>>();

    thread::Builder::new()
        .name("persistence-db".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ =
                        ready_tx.send(Err(anyhow!("failed to start persistence runtime: {error}")));
                    return;
                }
            };

            if let Err(error) =
                runtime.block_on(worker_main(database_url, request_rx, response_tx, ready_tx))
            {
                error!(?error, "persistence worker stopped");
            }
        })
        .context("failed to spawn persistence worker thread")?;

    ready_rx
        .recv()
        .context("persistence worker failed before becoming ready")??;

    Ok(PersistenceChannels {
        request_tx,
        response_rx: std::sync::Mutex::new(response_rx),
    })
}

async fn worker_main(
    database_url: String,
    request_rx: std::sync::mpsc::Receiver<PersistenceRequest>,
    response_tx: std::sync::mpsc::Sender<PersistenceResponse>,
    ready_tx: std::sync::mpsc::Sender<Result<()>>,
) -> Result<()> {
    let pool = match connect_database(&database_url).await {
        Ok(pool) => {
            let _ = ready_tx.send(Ok(()));
            pool
        }
        Err(error) => {
            let _ = ready_tx.send(Err(anyhow!(error.to_string())));
            return Err(error);
        }
    };

    let mut connection = pool
        .acquire()
        .await
        .context("failed to acquire dedicated persistence connection")?;

    while let Ok(request) = request_rx.recv() {
        match request {
            PersistenceRequest::LoadPlayer { username } => {
                let result = load_player_state(&mut connection, &username).await;
                let _ = response_tx.send(PersistenceResponse::LoadPlayer {
                    username,
                    result: result.map_err(|error| error.to_string()),
                });
            }
            PersistenceRequest::Flush {
                flush_id,
                transactions,
                usernames,
            } => {
                let result = flush_transactions(&mut connection, &transactions, &usernames).await;
                let _ = response_tx.send(PersistenceResponse::Flush {
                    flush_id,
                    result: result.map_err(|error| error.to_string()),
                });
            }
        }
    }

    Ok(())
}

async fn connect_database(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await
        .context("failed to connect to database")
}

async fn load_player_state(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    username: &str,
) -> Result<PersistentState> {
    let mut states = get_player_states(connection, &[username.to_string()]).await?;
    states
        .remove(username)
        .with_context(|| format!("get_player_states returned no row for `{username}`"))
}

async fn get_player_states(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    usernames: &[String],
) -> Result<HashMap<String, PersistentState>> {
    if usernames.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query("SELECT username, state FROM get_player_states($1)")
        .bind(usernames)
        .fetch_all(&mut **connection)
        .await
        .context("get_player_states failed")?;

    let mut states = HashMap::with_capacity(rows.len());
    for row in rows {
        let username: String = row.try_get("username")?;
        let state: serde_json::Value = row.try_get("state")?;
        let state: PersistentState =
            serde_json::from_value(state).context("invalid player state JSON")?;
        states.insert(username, state);
    }
    Ok(states)
}

async fn flush_transactions(
    connection: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    transactions: &[Transaction],
    usernames: &[String],
) -> Result<FlushResult> {
    let transactions_json =
        serde_json::to_value(transactions).context("failed to serialize transactions")?;

    let rows = sqlx::query("SELECT username, state_hash FROM flush_player_transactions($1, $2)")
        .bind(transactions_json)
        .bind(usernames)
        .fetch_all(&mut **connection)
        .await
        .context("flush_player_transactions failed")?;

    let mut hashes = HashMap::with_capacity(rows.len());
    for row in rows {
        let username: String = row.try_get("username")?;
        let state_hash: String = row.try_get("state_hash")?;
        hashes.insert(username, state_hash);
    }
    Ok(FlushResult { hashes })
}

#[derive(Resource)]
pub struct PersistenceFlushClock {
    pub next_flush_at: Instant,
    pub next_flush_id: u64,
    pub awaiting_flush: bool,
    pub flush_interval: Duration,
}
