use std::collections::{HashMap, HashSet};

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
use super::worker::{FlushResult, PersistenceChannels, PersistenceRequest, PersistenceResponse};

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
            PersistenceResponse::Flush { result } => {
                handle_flush_response(&mut queue, &mut ready_players, result);
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
    queue: &mut PendingTransactionQueue,
    ready_players: &mut Query<
        (Entity, &PersistenceIdentity, &mut CachedPersistentState),
        Without<PersistenceLoading>,
    >,
    result: Result<FlushResult, String>,
) {
    if !queue.is_flushing() {
        warn!("received unexpected flush response");
        return;
    }

    match result {
        Ok(flush) => {
            queue.acknowledge(&flush.acknowledged_ids);

            for (username, state) in flush.reloaded_states {
                let Some((_, _, mut cache)) = ready_players
                    .iter_mut()
                    .find(|(_, identity, _)| identity.username == username)
                else {
                    warn!(%username, "flush reload for unknown player");
                    continue;
                };

                warn!(
                    %username,
                    "persistent state hash mismatch; reloaded from database"
                );
                reconcile_reloaded_state(queue, &mut cache, username, Ok(state));
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
    players: Query<(&PersistenceIdentity, &CachedPersistentState), With<PersistenceReady>>,
) {
    let Some(transactions) = queue.begin_flush_if_due() else {
        return;
    };

    let connected_states: HashMap<String, PersistentState> = players
        .iter()
        .map(|(identity, cache)| (identity.username.clone(), cache.0.clone()))
        .collect();
    let usernames: Vec<String> = connected_states.keys().cloned().collect();
    let expected_hashes: HashMap<String, String> = connected_states
        .iter()
        .map(|(username, state)| (username.clone(), state.hash_hex(username)))
        .collect();

    if transactions.is_empty() && usernames.is_empty() {
        queue.finish_flush();
        return;
    }

    if channels
        .request_tx
        .send(PersistenceRequest::Flush {
            transactions,
            usernames,
            expected_hashes,
        })
        .is_err()
    {
        error!("persistence worker is unavailable");
        queue.finish_flush();
    }
}
