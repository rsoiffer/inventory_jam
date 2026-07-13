use std::collections::HashMap;
use std::thread;

use anyhow::{Context, Result, anyhow};
use bevy::prelude::*;
use sqlx::{Connection, PgConnection, Row};
use uuid::Uuid;

use super::model::{PersistentState, Transaction};

pub enum PersistenceRequest {
    LoadPlayer {
        username: String,
    },
    Flush {
        transactions: Vec<Transaction>,
        expected_hashes: HashMap<String, String>,
    },
}

pub enum PersistenceResponse {
    LoadPlayer {
        username: String,
        result: Result<PersistentState, String>,
    },
    Flush {
        result: Result<FlushResult, String>,
    },
}

#[derive(Debug, Clone)]
pub struct TransactionResult {
    pub id: Uuid,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FlushResult {
    pub transaction_results: Vec<TransactionResult>,
    pub reloaded_states: HashMap<String, PersistentState>,
}

#[derive(Resource)]
pub struct PersistenceChannels {
    request_tx: std::sync::mpsc::Sender<PersistenceRequest>,
    response_rx: std::sync::Mutex<std::sync::mpsc::Receiver<PersistenceResponse>>,
}

impl PersistenceChannels {
    pub fn load_player(&self, username: String) -> Result<(), String> {
        self.request_tx
            .send(PersistenceRequest::LoadPlayer { username })
            .map_err(|_| "persistence worker is unavailable".into())
    }

    pub fn flush(
        &self,
        transactions: Vec<Transaction>,
        expected_hashes: HashMap<String, String>,
    ) -> Result<(), String> {
        self.request_tx
            .send(PersistenceRequest::Flush {
                transactions,
                expected_hashes,
            })
            .map_err(|_| "persistence worker is unavailable".into())
    }

    pub fn drain_responses(&self) -> Vec<PersistenceResponse> {
        self.response_rx
            .lock()
            .unwrap_or_else(|poisoned| {
                error!("persistence response channel was poisoned");
                poisoned.into_inner()
            })
            .try_iter()
            .collect()
    }
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
    let mut connection = match connect_database(&database_url).await {
        Ok(connection) => {
            let _ = ready_tx.send(Ok(()));
            connection
        }
        Err(error) => {
            let _ = ready_tx.send(Err(anyhow!(error.to_string())));
            return Err(error);
        }
    };

    while let Ok(request) = request_rx.recv() {
        match request {
            PersistenceRequest::LoadPlayer { username } => {
                let result = load_player_state(&mut connection, &username).await;
                let _ = response_tx.send(PersistenceResponse::LoadPlayer {
                    username,
                    result: result.map_err(|error| format!("{error:#}")),
                });
            }
            PersistenceRequest::Flush {
                transactions,
                expected_hashes,
            } => {
                let result =
                    flush_and_reload(&mut connection, &transactions, &expected_hashes).await;
                let _ = response_tx.send(PersistenceResponse::Flush {
                    result: result.map_err(|error| format!("{error:#}")),
                });
            }
        }
    }

    Ok(())
}

async fn connect_database(database_url: &str) -> Result<PgConnection> {
    PgConnection::connect(database_url)
        .await
        .context("failed to connect to database")
}

async fn load_player_state(
    connection: &mut PgConnection,
    username: &str,
) -> Result<PersistentState> {
    let mut states = get_player_states(connection, &[username.to_string()]).await?;
    states
        .remove(username)
        .with_context(|| format!("get_player_states returned no row for `{username}`"))
}

async fn get_player_states(
    connection: &mut PgConnection,
    usernames: &[String],
) -> Result<HashMap<String, PersistentState>> {
    if usernames.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query("SELECT username, state FROM get_player_states($1)")
        .bind(usernames)
        .fetch_all(&mut *connection)
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

async fn flush_and_reload(
    connection: &mut PgConnection,
    transactions: &[Transaction],
    expected_hashes: &HashMap<String, String>,
) -> Result<FlushResult> {
    let transactions_json =
        serde_json::to_value(transactions).context("failed to serialize transactions")?;

    let usernames: Vec<_> = expected_hashes.keys().cloned().collect();
    let rows = sqlx::query(
        "SELECT transaction_id, applied, error_message, username, state_hash \
         FROM flush_player_transactions($1, $2)",
    )
    .bind(transactions_json)
    .bind(&usernames)
    .fetch_all(&mut *connection)
    .await
    .context("flush_player_transactions failed")?;

    let mut transaction_results = Vec::with_capacity(transactions.len());
    let mut actual_hashes = HashMap::with_capacity(expected_hashes.len());
    for row in rows {
        if let Some(id) = row.try_get::<Option<Uuid>, _>("transaction_id")? {
            let applied = row.try_get::<Option<bool>, _>("applied")? == Some(true);
            let error = row.try_get::<Option<String>, _>("error_message")?;
            transaction_results.push(TransactionResult {
                id,
                error: (!applied)
                    .then(|| error.unwrap_or_else(|| "PostgreSQL rejected the transaction".into())),
            });
        } else if let (Some(username), Some(state_hash)) = (
            row.try_get::<Option<String>, _>("username")?,
            row.try_get::<Option<String>, _>("state_hash")?,
        ) {
            actual_hashes.insert(username, state_hash);
        }
    }

    let mismatches: Vec<String> = expected_hashes
        .iter()
        .filter_map(|(username, expected)| match actual_hashes.get(username) {
            Some(actual) if actual == expected => None,
            Some(_) | None => Some(username.clone()),
        })
        .collect();

    let reloaded_states = get_player_states(connection, &mismatches).await?;

    Ok(FlushResult {
        transaction_results,
        reloaded_states,
    })
}
