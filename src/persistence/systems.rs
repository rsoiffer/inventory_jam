use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use lightyear::connection::client_of::ClientOf;
use lightyear::connection::host::HostServer;
use lightyear::prelude::*;

use crate::app::validate_username;
use crate::player::PlayerUsername;
use crate::protocol::messages::SetUsername;

use super::api::{PersistenceIdentity, PersistenceLoading, PersistenceReady};
use super::queue::PendingTransactionQueue;
use super::state::{CachedPersistentState, PersistentState};
use super::transaction::Transaction;
use super::worker::{
    FlushResult, PersistenceChannels, PersistenceFlushClock, PersistenceRequest,
    PersistenceResponse,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_username_messages(
    mut commands: Commands,
    mut receivers: Query<(Entity, &mut MessageReceiver<SetUsername>), With<ClientOf>>,
    mut players: Query<(
        Entity,
        &ControlledBy,
        &mut PlayerUsername,
        Option<&PersistenceIdentity>,
        Option<&PersistenceLoading>,
        Option<&PersistenceReady>,
        Has<Predicted>,
    )>,
    loading: Query<&PersistenceLoading>,
    identities: Query<&PersistenceIdentity>,
    channels: Res<PersistenceChannels>,
    host_server: Query<(), With<HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (link_entity, mut receiver) in &mut receivers {
        for message in receiver.receive() {
            let Ok(name) = validate_username(&message.name) else {
                warn!(
                    "ignored invalid username from {:?}: {:?}",
                    link_entity, message.name
                );
                continue;
            };

            let Some((player_entity, _, mut username, identity, loading_state, ready, _)) = players
                .iter_mut()
                .find(|(_, controlled_by, _, _, _, _, predicted)| {
                    controlled_by.owner == link_entity && !(is_host_server && *predicted)
                })
            else {
                warn!("no authoritative player for username from {link_entity:?}");
                continue;
            };

            if identity.is_some() || ready.is_some() || loading_state.is_some() {
                warn!(
                    "ignored username change for {player_entity:?}; identity is immutable after first set"
                );
                continue;
            }

            if identities.iter().any(|existing| existing.username == name)
                || loading.iter().any(|existing| existing.username == name)
            {
                warn!("rejected duplicate username `{name}` for {player_entity:?}");
                continue;
            }

            if channels
                .request_tx
                .send(PersistenceRequest::LoadPlayer {
                    username: name.clone(),
                })
                .is_err()
            {
                error!("persistence worker is unavailable");
                continue;
            }

            username.0 = name.clone();
            commands.entity(player_entity).insert(PersistenceLoading {
                username: name.clone(),
            });
            info!("loading persistent state for {player_entity:?} as {name}");
        }
    }
}

pub(crate) fn poll_persistence_responses(
    mut commands: Commands,
    channels: Res<PersistenceChannels>,
    mut queue: ResMut<PendingTransactionQueue>,
    mut clock: ResMut<PersistenceFlushClock>,
    mut loading_players: Query<
        (
            Entity,
            &PersistenceLoading,
            &mut PlayerUsername,
            Option<&mut CachedPersistentState>,
        ),
        With<PersistenceLoading>,
    >,
    mut ready_players: Query<
        (Entity, &PersistenceIdentity, &mut CachedPersistentState),
        Without<PersistenceLoading>,
    >,
) {
    while let Ok(response) = channels
        .response_rx
        .lock()
        .expect("persistence response channel poisoned")
        .try_recv()
    {
        match response {
            PersistenceResponse::LoadPlayer { username, result } => {
                handle_load_player_response(
                    &mut commands,
                    &queue,
                    &mut loading_players,
                    &mut ready_players,
                    username,
                    result,
                );
            }
            PersistenceResponse::Flush { flush_id, result } => {
                handle_flush_response(&channels, &mut queue, &mut clock, flush_id, result);
            }
        }
    }
}

fn handle_load_player_response(
    commands: &mut Commands,
    queue: &PendingTransactionQueue,
    loading_players: &mut Query<
        (
            Entity,
            &PersistenceLoading,
            &mut PlayerUsername,
            Option<&mut CachedPersistentState>,
        ),
        With<PersistenceLoading>,
    >,
    ready_players: &mut Query<
        (Entity, &PersistenceIdentity, &mut CachedPersistentState),
        Without<PersistenceLoading>,
    >,
    username: String,
    result: Result<PersistentState, String>,
) {
    if let Some((player_entity, _, mut player_username, cache)) = loading_players
        .iter_mut()
        .find(|(_, loading, _, _)| loading.username == username)
    {
        apply_initial_load(
            commands,
            player_entity,
            &mut player_username,
            cache,
            username,
            result,
        );
        return;
    }

    let Some((_, _, mut cache)) = ready_players
        .iter_mut()
        .find(|(_, identity, _)| identity.username == username)
    else {
        warn!(%username, "load response for unknown player");
        return;
    };

    reconcile_reloaded_state(queue, &mut cache, username, result);
}

fn apply_initial_load(
    commands: &mut Commands,
    player_entity: Entity,
    player_username: &mut PlayerUsername,
    cache: Option<Mut<CachedPersistentState>>,
    username: String,
    result: Result<PersistentState, String>,
) {
    match result {
        Ok(state) => {
            player_username.0 = username.clone();
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
                .insert((
                    PersistenceIdentity {
                        username: username.clone(),
                    },
                    PersistenceReady,
                ));
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

fn reconcile_reloaded_state(
    queue: &PendingTransactionQueue,
    cache: &mut CachedPersistentState,
    username: String,
    result: Result<PersistentState, String>,
) {
    match result {
        Ok(state) => {
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
        Err(error) => {
            error!(%username, %error, "failed to reload persistent state");
        }
    }
}

fn handle_flush_response(
    channels: &PersistenceChannels,
    queue: &mut PendingTransactionQueue,
    clock: &mut PersistenceFlushClock,
    flush_id: u64,
    result: Result<FlushResult, String>,
) {
    if !clock.awaiting_flush {
        warn!(flush_id, "received unexpected flush response");
        return;
    }
    clock.awaiting_flush = false;
    clock.schedule_next();

    match result {
        Ok(flush) => {
            let mismatches: Vec<String> = queue
                .flush_boundary_hashes
                .iter()
                .filter_map(|(username, expected)| match flush.hashes.get(username) {
                    Some(actual) if actual == expected => None,
                    Some(_) | None => Some(username.clone()),
                })
                .collect();

            queue.clear_in_flight();

            for username in mismatches {
                warn!(
                    %username,
                    "persistent state hash mismatch; reloading from database"
                );
                request_load(channels, username);
            }
        }
        Err(error) => {
            if error.contains("flush_player_transactions failed") {
                error!(
                    %error,
                    "flush rejected a transaction; abandoning in-flight work and reloading touched players"
                );
                let touched: HashSet<String> = queue
                    .in_flight
                    .iter()
                    .flat_map(|transaction| transaction.usernames().map(str::to_owned))
                    .collect();
                queue.abandon_in_flight();
                for username in touched {
                    request_load(channels, username);
                }
            } else {
                error!(
                    %error,
                    "transient flush failure; will retry in-flight transactions"
                );
            }
        }
    }
}

fn request_load(channels: &PersistenceChannels, username: String) {
    let _ = channels
        .request_tx
        .send(PersistenceRequest::LoadPlayer { username });
}

pub(crate) fn apply_persistence_transactions(
    mut messages: MessageReader<Transaction>,
    mut players: Query<(&PersistenceIdentity, &mut CachedPersistentState), With<PersistenceReady>>,
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
    players: &mut Query<(&PersistenceIdentity, &mut CachedPersistentState), With<PersistenceReady>>,
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
            .find(|(identity, _)| identity.username == player_change.username)
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
            .find(|(identity, _)| identity.username == username)
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
    mut clock: ResMut<PersistenceFlushClock>,
    players: Query<(&PersistenceIdentity, &CachedPersistentState), With<PersistenceReady>>,
) {
    if clock.awaiting_flush || Instant::now() < clock.next_flush_at {
        return;
    }

    let connected_states: HashMap<String, PersistentState> = players
        .iter()
        .map(|(identity, cache)| (identity.username.clone(), cache.0.clone()))
        .collect();
    let usernames: Vec<String> = connected_states.keys().cloned().collect();

    if queue.pending.is_empty() && queue.in_flight.is_empty() && usernames.is_empty() {
        clock.schedule_next();
        return;
    }

    if !queue.in_flight.is_empty() {
        let flush_id = clock.take_flush_id();
        let transactions = queue.in_flight.iter().cloned().collect::<Vec<_>>();
        if channels
            .request_tx
            .send(PersistenceRequest::Flush {
                flush_id,
                transactions,
                usernames,
            })
            .is_err()
        {
            error!("persistence worker is unavailable");
            return;
        }
        clock.awaiting_flush = true;
        return;
    }

    let (transactions, _) = queue.begin_flush(&connected_states);
    let flush_id = clock.take_flush_id();

    if channels
        .request_tx
        .send(PersistenceRequest::Flush {
            flush_id,
            transactions,
            usernames,
        })
        .is_err()
    {
        error!("persistence worker is unavailable");
        let stalled = queue.in_flight.drain(..).collect::<Vec<_>>();
        queue.pending.extend(stalled);
        queue.flush_boundary_hashes.clear();
        return;
    }

    clock.awaiting_flush = true;
}

impl PersistenceFlushClock {
    pub fn new(flush_interval: Duration) -> Self {
        Self {
            next_flush_at: Instant::now() + flush_interval,
            next_flush_id: 1,
            awaiting_flush: false,
            flush_interval,
        }
    }

    fn schedule_next(&mut self) {
        self.next_flush_at = Instant::now() + self.flush_interval;
    }

    fn take_flush_id(&mut self) -> u64 {
        let flush_id = self.next_flush_id;
        self.next_flush_id = self.next_flush_id.saturating_add(1);
        flush_id
    }
}
