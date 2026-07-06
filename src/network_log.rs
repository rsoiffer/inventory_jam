use bevy::prelude::*;
use lightyear::connection::client::{Connected, Connecting, Disconnected, Disconnecting};
use lightyear::prelude::*;
use lightyear::prelude::server::{Started, Starting, Stopped};

pub struct NetworkLoggingPlugin;

impl Plugin for NetworkLoggingPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "client")]
        {
            app.add_observer(log_client_connecting)
                .add_observer(log_client_connected)
                .add_observer(log_client_disconnecting)
                .add_observer(log_client_disconnected);
        }

        #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
        {
            app.add_observer(log_server_starting)
                .add_observer(log_server_started)
                .add_observer(log_server_stopped)
                .add_observer(log_incoming_client_link)
                .add_observer(log_client_disconnected_on_server);
        }
    }
}

#[cfg(feature = "client")]
fn log_client_connecting(
    trigger: On<Add, Connecting>,
    clients: Query<(&Name, Option<&PeerAddr>), With<Client>>,
) {
    let Ok((name, peer_addr)) = clients.get(trigger.entity) else {
        return;
    };
    match peer_addr {
        Some(peer_addr) => info!(
            "[net] CLIENT CONNECTING: {name} -> {}",
            peer_addr.0
        ),
        None => info!("[net] CLIENT CONNECTING: {name} (host client)"),
    }
}

#[cfg(feature = "client")]
fn log_client_connected(
    trigger: On<Add, Connected>,
    clients: Query<(&Name, &RemoteId), With<Client>>,
) {
    let Ok((name, remote_id)) = clients.get(trigger.entity) else {
        return;
    };
    info!("[net] CLIENT CONNECTED: {name} (remote peer {remote_id:?})");
}

#[cfg(feature = "client")]
fn log_client_disconnecting(
    trigger: On<Add, Disconnecting>,
    clients: Query<&Name, With<Client>>,
) {
    let Ok(name) = clients.get(trigger.entity) else {
        return;
    };
    info!("[net] CLIENT DISCONNECTING: {name}");
}

#[cfg(feature = "client")]
fn log_client_disconnected(
    trigger: On<Add, Disconnected>,
    clients: Query<(&Name, &Disconnected), With<Client>>,
) {
    let Ok((name, disconnected)) = clients.get(trigger.entity) else {
        return;
    };
    match &disconnected.reason {
        Some(reason) => info!("[net] CLIENT DISCONNECTED: {name} — {reason}"),
        None => info!("[net] CLIENT DISCONNECTED: {name}"),
    }
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn log_server_starting(trigger: On<Add, Starting>, servers: Query<&Name, With<Server>>) {
    let Ok(name) = servers.get(trigger.entity) else {
        return;
    };
    info!("[net] SERVER STARTING: {name}");
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn log_server_started(
    trigger: On<Add, Started>,
    servers: Query<(&Name, &LocalAddr), With<Server>>,
) {
    let Ok((name, local_addr)) = servers.get(trigger.entity) else {
        return;
    };
    info!("[net] SERVER LISTENING: {name} on {}", local_addr.0);
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn log_server_stopped(trigger: On<Add, Stopped>, servers: Query<&Name, With<Server>>) {
    let Ok(name) = servers.get(trigger.entity) else {
        return;
    };
    info!("[net] SERVER STOPPED: {name}");
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn log_incoming_client_link(
    trigger: On<Add, LinkOf>,
    clients: Query<(&Name, Option<&RemoteId>), With<LinkOf>>,
) {
    let Ok((name, remote_id)) = clients.get(trigger.entity) else {
        return;
    };
    match remote_id {
        Some(remote_id) => info!("[net] INCOMING CLIENT LINK: {name} ({remote_id:?})"),
        None => info!("[net] INCOMING CLIENT LINK: {name}"),
    }
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn log_client_disconnected_on_server(
    trigger: On<Add, Disconnected>,
    clients: Query<(&Name, &Disconnected, Option<&RemoteId>), With<LinkOf>>,
) {
    let Ok((name, disconnected, remote_id)) = clients.get(trigger.entity) else {
        return;
    };
    let peer = remote_id
        .map(|id| format!("{id:?}"))
        .unwrap_or_else(|| "unknown peer".to_string());
    match &disconnected.reason {
        Some(reason) => info!("[net] CLIENT LEFT: {name} ({peer}) — {reason}"),
        None => info!("[net] CLIENT LEFT: {name} ({peer})"),
    }
}
