use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use lightyear::connection::client_of::ClientOf;
use lightyear::connection::host::HostServer;
use lightyear::prelude::*;

use crate::app::validate_username;
use crate::player::PlayerUsername;
use crate::protocol::messages::SetUsername;

use super::api::{AddFragments, PersistenceIdentity, PersistenceLoading, PersistenceReady};
use super::queue::PersistenceQueue;
use super::state::{CachedPersistentState, PersistentState};
use super::transaction::PersistenceTransaction;
use super::worker::{
    PersistenceChannels, PersistenceFlushClock, PersistenceRequest, PersistenceResponse,
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
    mut clock: ResMut<PersistenceFlushClock>,
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

            let request_id = clock.next_request_id;
            clock.next_request_id = clock.next_request_id.saturating_add(1);

            if channels
                .request_tx
                .send(PersistenceRequest::LoadPlayer {
                    username: name.clone(),
                    request_id,
                })
                .is_err()
            {
                error!("persistence worker is unavailable");
                continue;
            }

            username.0 = name.clone();
            commands.entity(player_entity).insert(PersistenceLoading {
                request_id,
                username: name.clone(),
            });
            info!("loading persistent state for {player_entity:?} as {name}");
        }
    }
}

pub(crate) fn poll_persistence_responses(
    mut commands: Commands,
    channels: Res<PersistenceChannels>,
    mut queue: ResMut<PersistenceQueue>,
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
            PersistenceResponse::LoadPlayer {
                request_id,
                username,
                result,
            } => {
                let Some((player_entity, loading, mut player_username, cache)) = loading_players
                    .iter_mut()
                    .find(|(_, loading, _, _)| loading.request_id == request_id)
                else {
                    warn!(
                        %username,
                        request_id,
                        "load response for unknown player"
                    );
                    continue;
                };

                if loading.username != username {
                    warn!(
                        expected = %loading.username,
                        got = %username,
                        "load response username mismatch"
                    );
                }

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
            PersistenceResponse::Flush { flush_id, result } => {
                if !clock.awaiting_flush {
                    warn!(flush_id, "received unexpected flush response");
                    continue;
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
                            request_reload(&channels, username);
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
                                .map(|transaction| transaction.username.clone())
                                .collect();
                            queue.abandon_in_flight();
                            for username in touched {
                                request_reload(&channels, username);
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
            PersistenceResponse::ReloadPlayer { username, result } => {
                let Some((_, _, mut cache)) = ready_players
                    .iter_mut()
                    .find(|(_, identity, _)| identity.username == username)
                else {
                    warn!(%username, "reload response for disconnected player");
                    continue;
                };

                match result {
                    Ok(state) => {
                        let mut next_state = state;
                        for transaction in queue.pending_for_username(&username) {
                            if let Err(error) = transaction.apply_to(&mut next_state) {
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
        }
    }
}

fn request_reload(channels: &PersistenceChannels, username: String) {
    let _ = channels
        .request_tx
        .send(PersistenceRequest::ReloadPlayer { username });
}

pub(crate) fn apply_add_fragments_messages(
    mut messages: MessageReader<AddFragments>,
    mut players: Query<(&PersistenceIdentity, &mut CachedPersistentState), With<PersistenceReady>>,
    mut queue: ResMut<PersistenceQueue>,
) {
    for message in messages.read() {
        let Ok((identity, mut cache)) = players.get_mut(message.player) else {
            warn!(
                player = ?message.player,
                amount = message.amount,
                "ignored AddFragments for player that is not persistence-ready"
            );
            continue;
        };

        let transaction =
            match PersistenceTransaction::add_fragments(&identity.username, message.amount) {
                Ok(transaction) => transaction,
                Err(error) => {
                    error!(
                        player = ?message.player,
                        username = %identity.username,
                        %error,
                        "rejected AddFragments transaction"
                    );
                    continue;
                }
            };

        if let Err(error) = transaction.apply_to(&mut cache.0) {
            error!(
                player = ?message.player,
                username = %identity.username,
                %error,
                "failed to apply AddFragments transaction"
            );
            continue;
        }

        queue.enqueue(transaction);
    }
}

pub(crate) fn flush_persistence_queue(
    channels: Res<PersistenceChannels>,
    mut queue: ResMut<PersistenceQueue>,
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
            next_request_id: 1,
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
