use core::net::{Ipv4Addr, SocketAddr};

use crate::config::{SERVER_PORT, webtransport_cert_path, webtransport_key_path};
use crate::debug_log::agent_log;
use crate::protocol::*;
use crate::shared::{SharedSettings, replication_metadata, shared_movement_behaviour};
use async_compat::Compat;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use lightyear::connection::client::Connected;
use lightyear::connection::host::HostServer;
use lightyear::prelude::input::native::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use lightyear::webtransport::prelude::Identity;
use lightyear::{netcode::NetcodeServer, webtransport::server::WebTransportServerIo};

pub struct ExampleServerPlugin;

impl Plugin for ExampleServerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PendingStart>()
            .register_type::<ServerBindAddr>()
            .insert_resource(replication_metadata());
        app.add_systems(FixedUpdate, (movement, debug_server_players).chain());
        app.add_observer(handle_new_client);
        app.add_observer(handle_connected);
        app.add_systems(Update, send_message);
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct PendingStart;

#[derive(Resource, Clone, Copy, Reflect)]
#[reflect(Resource)]
pub(crate) struct ServerBindAddr(
    #[reflect(ignore, default = "default_bind_addr")]
    pub SocketAddr,
);

fn default_bind_addr() -> SocketAddr {
    SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0)
}

pub fn maybe_spawn_server(
    mut commands: Commands,
    session: Option<Res<crate::config::NetworkSession>>,
    server: Query<Entity, With<Server>>,
) -> Result {
    let Some(session) = session else {
        return Ok(());
    };
    if session.mode != crate::config::SessionMode::Host || !server.is_empty() {
        return Ok(());
    }

    let shared = SharedSettings::default();
    let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), SERVER_PORT);
    let identity = IoTaskPool::get()
        .scope(|scope| {
            scope.spawn(Compat::new(async {
                Identity::load_pemfiles(&webtransport_cert_path(), &webtransport_key_path())
                    .await
                    .expect("failed to load WebTransport certificate")
            }));
        })
        .pop()
        .expect("certificate load task missing");

    info!("[net] Spawning embedded server on {server_addr}");

    commands.spawn((
        Name::new("Server"),
        NetcodeServer::new(NetcodeConfig {
            protocol_id: shared.protocol_id,
            private_key: shared.private_key,
            ..Default::default()
        }),
        LocalAddr(server_addr),
        WebTransportServerIo {
            certificate: identity,
        },
        PendingStart,
    ));
    Ok(())
}

pub fn start_pending_servers(
    mut commands: Commands,
    servers: Query<(Entity, &Name), With<PendingStart>>,
) {
    for (entity, name) in &servers {
        info!("[net] Starting server {name}");
        commands.trigger(Start { entity });
        commands.entity(entity).remove::<PendingStart>();
    }
}

pub fn spawn_dedicated_server(
    mut commands: Commands,
    bind_addr: Option<Res<ServerBindAddr>>,
) -> Result {
    let shared = SharedSettings::default();
    let server_addr = bind_addr
        .map(|bind_addr| bind_addr.0)
        .unwrap_or_else(|| SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), SERVER_PORT));
    let identity = IoTaskPool::get()
        .scope(|scope| {
            scope.spawn(Compat::new(async {
                Identity::load_pemfiles(&webtransport_cert_path(), &webtransport_key_path())
                    .await
                    .expect("failed to load WebTransport certificate")
            }));
        })
        .pop()
        .expect("certificate load task missing");

    info!("[net] Spawning dedicated server on {server_addr}");

    commands.spawn((
        Name::new("Server"),
        NetcodeServer::new(NetcodeConfig {
            protocol_id: shared.protocol_id,
            private_key: shared.private_key,
            ..Default::default()
        }),
        LocalAddr(server_addr),
        WebTransportServerIo {
            certificate: identity,
        },
        PendingStart,
    ));
    Ok(())
}

fn handle_new_client(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert((ReplicationSender, Name::from("Client")));
}

fn handle_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<LinkOf>>,
    mut commands: Commands,
) {
    let Ok(client_id) = query.get(trigger.entity) else {
        return;
    };
    let client_id = client_id.0;
    let entity = commands
        .spawn((
            PlayerBundle::new(client_id, Vec2::ZERO),
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::Single(client_id)),
            InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(client_id)),
            ControlledBy {
                owner: trigger.entity,
                lifetime: Default::default(),
            },
        ))
        .id();
    info!(
        "[net] PLAYER SPAWNED: {entity:?} for client {client_id:?} (connection {:?})",
        trigger.entity
    );
}

fn debug_server_players(
    timeline: Res<LocalTimeline>,
    server: Query<(), (With<Server>, With<Started>)>,
    host_server: Query<(), With<HostServer>>,
    players: Query<(Entity, &PlayerPosition, &ActionState<Inputs>, Has<Predicted>)>,
) {
    if server.is_empty() || timeline.tick().0 % 32 != 0 {
        return;
    }
    let is_host_server = !host_server.is_empty();
    for (entity, position, inputs, predicted) in &players {
        let skipped = is_host_server && predicted;
        // #region agent log
        agent_log(
            "M",
            "server.rs:debug_server_players",
            "server player snapshot",
            &format!(
                r#"{{"tick":{},"entity":{:?},"predicted":{},"skipped":{},"pos_x":{:.4},"pos_y":{:.4},"input_x":{:.4},"input_y":{:.4}}}"#,
                timeline.tick().0,
                entity,
                predicted,
                skipped,
                position.0.x,
                position.0.y,
                inputs.0.x,
                inputs.0.y,
            ),
        );
        // #endregion
    }
}

fn movement(
    timeline: Res<LocalTimeline>,
    server: Query<(), (With<Server>, With<Started>)>,
    host_server: Query<(), With<HostServer>>,
    mut position_query: Query<(Entity, &mut PlayerPosition, &ActionState<Inputs>, Has<Predicted>)>,
) {
    if server.is_empty() {
        return;
    }
    let is_host_server = !host_server.is_empty();
    let tick = timeline.tick().0;
    for (entity, mut position, inputs, predicted) in position_query.iter_mut() {
        if is_host_server && predicted {
            continue;
        }
        if inputs.0.length_squared() == 0.0 {
            continue;
        }
        let before = position.0;
        shared_movement_behaviour(position, inputs);
        let after_x = before.x + inputs.0.x * 10.0;
        // #region agent log
        agent_log(
            "G",
            "server.rs:movement",
            "server movement applied",
            &format!(
                r#"{{"entity":{:?},"tick":{},"predicted":{},"input_x":{:.4},"input_y":{:.4},"before_x":{:.4},"after_x":{:.4}}}"#,
                entity,
                tick,
                predicted,
                inputs.0.x,
                inputs.0.y,
                before.x,
                after_x,
            ),
        );
        // #endregion
    }
}

fn send_message(
    mut sender: ServerMultiMessageSender,
    server: Single<&Server>,
    input: Option<Res<ButtonInput<KeyCode>>>,
) {
    if input.is_some_and(|input| input.just_pressed(KeyCode::KeyM)) {
        let message = Message1(5);
        info!("Sending message: {message:?}");
        sender
            .send::<_, Channel1>(&message, server.into_inner(), &NetworkTarget::All)
            .unwrap_or_else(|error| error!("Failed to send message: {error:?}"));
    }
}
