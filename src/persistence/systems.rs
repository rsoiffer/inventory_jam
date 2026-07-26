use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use lightyear::connection::client_of::ClientOf;
use lightyear::prelude::*;
use uuid::Uuid;

use crate::app::validate_username;
use crate::player::PlayerUsername;
use crate::protocol::messages::SetUsername;

use super::model::{
    CachedPersistentState, PersistenceLoading, PersistenceReady, PersistentState, Transaction,
};
use super::worker::{FlushResult, PersistenceChannels, PersistenceResponse};

const FLUSH_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Resource)]
pub(crate) struct PendingTransactionQueue {
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
    fn enqueue(&mut self, transaction: Transaction) {
        self.pending.push_back(transaction);
    }

    fn is_flushing(&self) -> bool {
        self.is_flushing
    }

    fn begin_flush_if_due(&mut self) -> Option<Vec<Transaction>> {
        if self.is_flushing || Instant::now() < self.next_flush_at {
            return None;
        }
        self.is_flushing = true;
        Some(self.pending.iter().cloned().collect())
    }

    fn acknowledge(&mut self, transaction_ids: &[Uuid]) {
        let ids: HashSet<Uuid> = transaction_ids.iter().copied().collect();
        self.pending
            .retain(|transaction| !ids.contains(&transaction.id));
    }

    fn pending_for_username<'a>(
        &'a self,
        username: &'a str,
    ) -> impl Iterator<Item = &'a Transaction> {
        self.pending
            .iter()
            .filter(move |transaction| transaction.touches_username(username))
    }

    fn finish_flush(&mut self) {
        self.is_flushing = false;
        self.next_flush_at = Instant::now() + FLUSH_INTERVAL;
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_username_messages(
    mut commands: Commands,
    mut receivers: Query<(Entity, &mut MessageReceiver<SetUsername>), With<ClientOf>>,
    mut players: Query<(
        Entity,
        &ControlledBy,
        &mut PlayerUsername,
        Option<&PersistenceLoading>,
        Option<&PersistenceReady>,
    )>,
    channels: Res<PersistenceChannels>,
) {
    for (link_entity, mut receiver) in &mut receivers {
        for message in receiver.receive() {
            let Ok(name) = validate_username(&message.name) else {
                warn!(
                    "ignored invalid username from {:?}: {:?}",
                    link_entity, message.name
                );
                continue;
            };

            let Some((player_entity, is_initialized)) = players
                .iter()
                .find(|(_, controlled_by, _, _, _)| controlled_by.owner == link_entity)
                .map(|(entity, _, _, loading, ready)| {
                    (entity, loading.is_some() || ready.is_some())
                })
            else {
                warn!("no authoritative player for username from {link_entity:?}");
                continue;
            };

            if is_initialized {
                warn!(
                    "ignored username change for {player_entity:?}; identity is immutable after first set"
                );
                continue;
            }

            if players.iter().any(|(entity, _, username, loading, ready)| {
                entity != player_entity
                    && (loading.is_some() || ready.is_some())
                    && username.0 == name
            }) {
                warn!("rejected duplicate username `{name}` for {player_entity:?}");
                continue;
            }

            if let Err(error) = channels.load_player(name.clone()) {
                error!(%error);
                continue;
            }

            let Ok((_, _, mut username, _, _)) = players.get_mut(player_entity) else {
                warn!("player {player_entity:?} disappeared before persistence load");
                continue;
            };
            username.0 = name.clone();
            commands.entity(player_entity).insert(PersistenceLoading);
            info!("loading persistent state for {player_entity:?} as {name}");
        }
    }
}

pub(crate) fn poll_persistence_responses(
    mut commands: Commands,
    channels: Res<PersistenceChannels>,
    mut queue: ResMut<PendingTransactionQueue>,
    mut loading_players: Query<
        (Entity, &PlayerUsername, Option<&mut CachedPersistentState>),
        With<PersistenceLoading>,
    >,
    mut ready_players: Query<
        (Entity, &PlayerUsername, &mut CachedPersistentState),
        (With<PersistenceReady>, Without<PersistenceLoading>),
    >,
) {
    for response in channels.drain_responses() {
        match response {
            PersistenceResponse::LoadPlayer { username, result } => {
                let Some((player_entity, _, cache)) = loading_players
                    .iter_mut()
                    .find(|(_, player_username, _)| player_username.0 == username)
                else {
                    warn!(%username, "load response for unknown player");
                    continue;
                };

                match result {
                    Ok(state) => {
                        match cache {
                            Some(mut cache) => cache.0 = state,
                            None => {
                                commands
                                    .entity(player_entity)
                                    .insert(CachedPersistentState(state));
                            }
                        }
                        commands
                            .entity(player_entity)
                            .remove::<PersistenceLoading>()
                            .insert(PersistenceReady);
                        info!(%username, "loaded persistent state for {player_entity:?}");
                    }
                    Err(error) => {
                        warn!(%username, %error, "failed to load persistent state");
                        commands
                            .entity(player_entity)
                            .remove::<PersistenceLoading>();
                    }
                }
            }
            PersistenceResponse::Flush { result } => {
                handle_flush_response(&mut queue, &mut ready_players, result);
            }
        }
    }
}

fn reconcile_reloaded_state(
    queue: &PendingTransactionQueue,
    cache: &mut CachedPersistentState,
    username: String,
    state: PersistentState,
) {
    let mut next_state = state;
    for transaction in queue.pending_for_username(&username) {
        if let Err(error) = transaction.apply_for_username(&username, &mut next_state) {
            error!(
                %username,
                %error,
                transaction_id = %transaction.id,
                "failed to reapply pending transaction after reload"
            );
        }
    }
    cache.0 = next_state;
    info!(%username, "reloaded and reconciled persistent state");
}

fn handle_flush_response(
    queue: &mut PendingTransactionQueue,
    ready_players: &mut Query<
        (Entity, &PlayerUsername, &mut CachedPersistentState),
        (With<PersistenceReady>, Without<PersistenceLoading>),
    >,
    result: Result<FlushResult, String>,
) {
    if !queue.is_flushing() {
        warn!("received unexpected flush response");
        return;
    }

    match result {
        Ok(flush) => {
            for transaction in &flush.transaction_results {
                if let Some(error) = &transaction.error {
                    warn!(
                        transaction_id = %transaction.id,
                        %error,
                        "PostgreSQL rejected persistent transaction"
                    );
                }
            }
            queue.acknowledge(
                &flush
                    .transaction_results
                    .iter()
                    .map(|transaction| transaction.id)
                    .collect::<Vec<_>>(),
            );

            for (username, state) in flush.reloaded_states {
                let Some((_, _, mut cache)) = ready_players
                    .iter_mut()
                    .find(|(_, player_username, _)| player_username.0 == username)
                else {
                    warn!(%username, "flush reload for unknown player");
                    continue;
                };

                warn!(
                    %username,
                    "persistent state hash mismatch; reloaded from database"
                );
                reconcile_reloaded_state(queue, &mut cache, username, state);
            }

            queue.finish_flush();
        }
        Err(error) => {
            error!(%error, "flush failed; will retry pending transactions");
            queue.finish_flush();
        }
    }
}

pub(crate) fn apply_persistence_transactions(
    mut messages: MessageReader<Transaction>,
    mut players: Query<(&PlayerUsername, &mut CachedPersistentState), With<PersistenceReady>>,
    mut queue: ResMut<PendingTransactionQueue>,
) {
    for message in messages.read() {
        match apply_transaction(message, &mut players) {
            Ok(()) => queue.enqueue(message.clone()),
            Err(error) => {
                error!(%error, transaction_id = %message.id, "rejected PersistenceTransaction")
            }
        }
    }
}

fn apply_transaction(
    transaction: &Transaction,
    players: &mut Query<(&PlayerUsername, &mut CachedPersistentState), With<PersistenceReady>>,
) -> Result<(), String> {
    if transaction.changes.is_empty() {
        return Err("transaction must include at least one change".into());
    }

    let mut seen_usernames = HashSet::new();
    let mut next_states = Vec::with_capacity(transaction.changes.len());

    for player_change in &transaction.changes {
        if !seen_usernames.insert(player_change.username.as_str()) {
            return Err(format!(
                "duplicate username `{}` in one transaction",
                player_change.username
            ));
        }

        let mut next_state = players
            .iter()
            .find(|(username, _)| username.0 == player_change.username)
            .map(|(_, cache)| cache.0.clone())
            .ok_or_else(|| {
                format!(
                    "player `{}` is not persistence-ready",
                    player_change.username
                )
            })?;
        player_change.apply(&mut next_state)?;
        next_states.push((player_change.username.clone(), next_state));
    }

    for (username, next_state) in next_states {
        let Some((_, mut cache)) = players
            .iter_mut()
            .find(|(player_username, _)| player_username.0 == username)
        else {
            return Err(format!("player `{username}` disappeared during apply"));
        };
        cache.0 = next_state;
    }

    Ok(())
}

pub(crate) fn flush_persistence_queue(
    channels: Res<PersistenceChannels>,
    mut queue: ResMut<PendingTransactionQueue>,
    players: Query<(&PlayerUsername, &CachedPersistentState), With<PersistenceReady>>,
) {
    let Some(transactions) = queue.begin_flush_if_due() else {
        return;
    };

    let expected_hashes: HashMap<String, String> = players
        .iter()
        .map(|(username, cache)| (username.0.clone(), cache.0.hash_hex(&username.0)))
        .collect();

    if transactions.is_empty() && expected_hashes.is_empty() {
        queue.finish_flush();
        return;
    }

    if let Err(error) = channels.flush(transactions, expected_hashes) {
        error!(%error);
        queue.finish_flush();
    }
}
