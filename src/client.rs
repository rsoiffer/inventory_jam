use core::net::{Ipv4Addr, SocketAddr};



use crate::actions::{Actions, movement_from_player};

use crate::config::{CLIENT_PORT, NetworkSession, SessionMode, webtransport_certificate_digest};

use crate::debug_log::agent_log;

use crate::protocol::*;

use crate::shared::{SharedSettings, shared_movement_behaviour};

use bevy::prelude::*;

use lightyear::netcode::client_plugin::NetcodeConfig;

use lightyear::prelude::client::input::*;

use lightyear::prelude::client::{InputDelayConfig, InputTimeline, InputTimelineConfig};

use lightyear::prelude::input::native::*;

use lightyear::prelude::*;

use lightyear::{netcode::NetcodeClient, webtransport::client::WebTransportClientIo};



pub struct ExampleClientPlugin;



impl Plugin for ExampleClientPlugin {

    fn build(&self, app: &mut App) {

        app.register_type::<PendingConnect>()

            .add_systems(

            FixedPreUpdate,

            buffer_input.in_set(InputSystems::WriteClientInputs),

        );

        app.add_systems(FixedUpdate, player_movement);

        app.add_systems(PostUpdate, debug_prediction_state);
        app.add_systems(PostUpdate, debug_interpolated_state);
        app.add_systems(PostUpdate, debug_input_pipeline);

        app.add_systems(Update, receive_message1);

        app.add_observer(handle_predicted_spawn);

        app.add_observer(handle_controlled_spawn);

        app.add_observer(handle_interpolated_spawn);

    }

}



#[derive(Component, Reflect, Default)]

#[reflect(Component)]

pub(crate) struct PendingConnect;



pub fn maybe_spawn_join_client(

    mut commands: Commands,

    session: Option<Res<NetworkSession>>,

    existing_clients: Query<Entity, (With<Client>, Without<LinkOf>)>,

) -> Result {

    let Some(session) = session else {

        return Ok(());

    };

    if session.mode != SessionMode::Join || !existing_clients.is_empty() {

        return Ok(());

    }



    let client_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), CLIENT_PORT);

    let shared = SharedSettings::default();

    let auth = Authentication::Manual {

        server_addr: session.server_addr,

        client_id: session.client_id,

        private_key: shared.private_key,

        protocol_id: shared.protocol_id,

    };



    info!(

        "[net] Spawning join client (id={}, server={})",

        session.client_id, session.server_addr

    );



    commands.spawn((

        Client::default(),

        Link::new(None),

        LocalAddr(client_addr),

        PeerAddr(session.server_addr),

        PredictionManager::default(),

        InputTimelineConfig::default().with_input_delay(InputDelayConfig::no_input_delay()),

        Name::new(format!("Client {}", session.client_id)),

        NetcodeClient::new(

            auth,

            NetcodeConfig {

                client_timeout_secs: 3,

                token_expire_secs: -1,

                ..default()

            },

        )?,

        WebTransportClientIo {

            certificate_digest: webtransport_certificate_digest(),

        },

        PendingConnect,

    ));

    Ok(())

}



#[cfg(all(feature = "server", not(target_arch = "wasm32")))]

pub fn maybe_spawn_host_client(

    mut commands: Commands,

    session: Option<Res<NetworkSession>>,

    server: Query<Entity, With<Server>>,

    host_clients: Query<Entity, With<LinkOf>>,

) -> Result {

    let Some(session) = session else {

        return Ok(());

    };

    if session.mode != SessionMode::Host || !host_clients.is_empty() {

        return Ok(());

    }

    let Ok(server) = server.single() else {

        return Ok(());

    };



    info!(

        "[net] Spawning host client (id={}) linked to server {:?}",

        session.client_id, server

    );



    commands.spawn((
        Client::default(),
        PredictionManager::default(),
        InputTimelineConfig::default().with_input_delay(InputDelayConfig::no_input_delay()),
        Name::new(format!("HostClient {}", session.client_id)),
        LinkOf { server },
        PendingConnect,
    ));

    Ok(())

}



pub fn connect_pending_clients(

    mut commands: Commands,

    clients: Query<(Entity, &Name), With<PendingConnect>>,

) {

    for (entity, name) in &clients {

        info!("[net] Requesting connection for {name}");

        commands.trigger(Connect { entity });

        commands.entity(entity).remove::<PendingConnect>();

    }

}



fn buffer_input(
    mut query: Query<(Entity, &mut ActionState<Inputs>), With<InputMarker<Inputs>>>,
    mut actions: ResMut<Actions>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    touch_input: Res<Touches>,
    player: Query<&PlayerPosition, (With<Predicted>, With<Controlled>)>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let Ok((entity, mut action_state)) = query.single_mut() else {
        return;
    };
    let movement = movement_from_player(
        &keyboard_input,
        &touch_input,
        player.single().ok(),
        camera.single().ok(),
    );
    actions.player_movement = (movement != Vec2::ZERO).then_some(movement);
    action_state.0 = Inputs(movement);
    if movement != Vec2::ZERO {
        // #region agent log
        agent_log(
            "I",
            "client.rs:buffer_input",
            "input buffered",
            &format!(
                r#"{{"entity":{:?},"input_x":{:.4},"input_y":{:.4}}}"#,
                entity, movement.x, movement.y,
            ),
        );
        // #endregion
    }
}



fn player_movement(

    synced_client: Query<(), (With<Client>, With<IsSynced<InputTimeline>>)>,

    time: Res<Time<Fixed>>,

    timeline: Res<LocalTimeline>,

    mut position_query: Query<
        (Entity, &mut PlayerPosition, &ActionState<Inputs>),
        (With<Predicted>, With<InputMarker<Inputs>>),
    >,

) {

    if synced_client.is_empty() {

        return;

    }

    let entity_count = position_query.iter().count();

    let tick = timeline.tick().0;

    let timestep_ms = time.timestep().as_secs_f64() * 1000.0;

    for (entity, mut position, input) in &mut position_query {

        if input.0.length_squared() == 0.0 {

            continue;

        }

        let before = position.0;
        shared_movement_behaviour(position, input);
        let after = before + input.0.0 * 10.0;

        // #region agent log

        agent_log(

            "A",

            "client.rs:player_movement",

            "movement applied",

            &format!(

                r#"{{"entity":{:?},"tick":{},"entity_count":{},"timestep_ms":{:.4},"input_x":{:.4},"input_y":{:.4},"before_x":{:.4},"before_y":{:.4},"after_x":{:.4},"after_y":{:.4},"delta_x":{:.4},"delta_y":{:.4}}}"#,

                entity,

                tick,

                entity_count,

                timestep_ms,

                input.0.x,

                input.0.y,

                before.x,

                before.y,

                after.x,

                after.y,

                after.x - before.x,

                after.y - before.y,

            ),

        );

        // #endregion

    }

}



fn debug_prediction_state(

    timeline: Res<LocalTimeline>,

    time: Res<Time<Fixed>>,

    predicted: Query<

        (

            Entity,

            &PlayerPosition,

            Option<&Transform>,

            Option<&VisualCorrection<PlayerPosition>>,

            Option<&ConfirmedHistory<PlayerPosition>>,

        ),

        With<Predicted>,

    >,

) {

    let tick = timeline.tick().0;

    let overstep = time.overstep_fraction();

    for (entity, predicted_pos, transform, correction, confirmed_history) in &predicted {

        let confirmed = confirmed_history

            .and_then(|h| h.newest_present())

            .map(|(_, pos)| pos.0);

        let gap_len = confirmed

            .map(|c| (predicted_pos.0 - c).length())

            .unwrap_or(0.0);

        let correction_err = correction.map(|c| c.error.0);

        let transform_pos = transform.map(|t| t.translation.truncate());

        let visual_gap = transform_pos.map(|t| t - predicted_pos.0);



        let should_log = gap_len > 2.0 || correction.is_some() || tick % 32 == 0;

        if !should_log {

            continue;

        }



        // #region agent log

        agent_log(

            if correction.is_some() {

                "C"

            } else if gap_len > 5.0 {

                "D"

            } else {

                "E"

            },

            "client.rs:debug_prediction_state",

            "prediction snapshot",

            &format!(

                r#"{{"entity":{:?},"tick":{},"overstep":{:.4},"predicted_x":{:.4},"predicted_y":{:.4},"confirmed_x":{},"confirmed_y":{},"gap_len":{:.4},"transform_x":{},"transform_y":{},"visual_gap_x":{},"visual_gap_y":{},"has_correction":{},"correction_err_x":{},"correction_err_y":{}}}"#,

                entity,

                tick,

                overstep,

                predicted_pos.0.x,

                predicted_pos.0.y,

                confirmed

                    .map(|c| format!("{:.4}", c.x))

                    .unwrap_or_else(|| "null".into()),

                confirmed

                    .map(|c| format!("{:.4}", c.y))

                    .unwrap_or_else(|| "null".into()),

                gap_len,

                transform_pos

                    .map(|t| format!("{:.4}", t.x))

                    .unwrap_or_else(|| "null".into()),

                transform_pos

                    .map(|t| format!("{:.4}", t.y))

                    .unwrap_or_else(|| "null".into()),

                visual_gap

                    .map(|g| format!("{:.4}", g.x))

                    .unwrap_or_else(|| "null".into()),

                visual_gap

                    .map(|g| format!("{:.4}", g.y))

                    .unwrap_or_else(|| "null".into()),

                correction.is_some(),

                correction_err

                    .map(|e| format!("{:.4}", e.x))

                    .unwrap_or_else(|| "null".into()),

                correction_err

                    .map(|e| format!("{:.4}", e.y))

                    .unwrap_or_else(|| "null".into()),

            ),

        );

        // #endregion

    }

}



fn debug_interpolated_state(
    timeline: Res<LocalTimeline>,
    interpolated: Query<(Entity, &PlayerPosition, Option<&Transform>), With<Interpolated>>,
) {
    let tick = timeline.tick().0;
    if tick % 32 != 0 {
        return;
    }
    for (entity, position, transform) in &interpolated {
        // #region agent log
        agent_log(
            "H",
            "client.rs:debug_interpolated_state",
            "interpolated snapshot",
            &format!(
                r#"{{"entity":{:?},"tick":{},"pos_x":{:.4},"pos_y":{:.4},"transform_x":{},"transform_y":{}}}"#,
                entity,
                tick,
                position.0.x,
                position.0.y,
                transform
                    .map(|t| format!("{:.4}", t.translation.x))
                    .unwrap_or_else(|| "null".into()),
                transform
                    .map(|t| format!("{:.4}", t.translation.y))
                    .unwrap_or_else(|| "null".into()),
            ),
        );
        // #endregion
    }
}



fn receive_message1(mut receiver: Single<&mut MessageReceiver<Message1>>) {

    for message in receiver.receive() {

        info!("Received message: {message:?}");

    }

}



fn handle_predicted_spawn(

    trigger: On<Add, (PlayerId, Predicted)>,

    mut predicted: Query<&mut PlayerColor, With<Predicted>>,

) {

    if let Ok(mut color) = predicted.get_mut(trigger.entity) {

        let hsva = Hsva {

            saturation: 0.4,

            ..Hsva::from(color.0)

        };

        color.0 = Color::from(hsva);

    }

}



fn handle_controlled_spawn(
    trigger: On<Add, Controlled>,
    mut commands: Commands,
    players: Query<(&PlayerId, &ControlledBy), Without<InputMarker<Inputs>>>,
    client: Query<Entity, (With<Client>, With<IsSynced<InputTimeline>>)>,
) {
    let Ok((player_id, controlled_by)) = players.get(trigger.entity) else {
        return;
    };
    let Ok(client_entity) = client.single() else {
        return;
    };
    if client.get(controlled_by.owner).is_err() {
        return;
    }

    if controlled_by.owner != client_entity {
        commands.entity(trigger.entity).insert(ControlledBy {
            owner: client_entity,
            lifetime: controlled_by.lifetime,
        });
    }

    info!("Adding input marker to controlled player {player_id:?}");
    commands
        .entity(trigger.entity)
        .insert(InputMarker::<Inputs>::default());
    // #region agent log
    agent_log(
        "J",
        "client.rs:handle_controlled_spawn",
        "input marker assigned",
        &format!(
            r#"{{"entity":{:?},"player_id":{},"client_entity":{:?},"owner_entity":{:?},"owner_matches_client":{}}}"#,
            trigger.entity,
            player_id.0.to_bits(),
            client_entity,
            controlled_by.owner,
            controlled_by.owner == client_entity,
        ),
    );
    // #endregion
}

fn debug_input_pipeline(
    timeline: Res<LocalTimeline>,
    client: Query<(Entity, Has<Rollback>), (With<Client>, With<IsSynced<InputTimeline>>)>,
    players: Query<(Entity, &ControlledBy), With<InputMarker<Inputs>>>,
    actions: Query<&ActionState<Inputs>, With<InputMarker<Inputs>>>,
) {
    let Ok((client_entity, in_rollback)) = client.single() else {
        return;
    };
    let tick = timeline.tick().0;
    if tick % 32 != 0 {
        return;
    }
    for (player, controlled_by) in &players {
        let Ok(action) = actions.get(player) else {
            continue;
        };
        if action.0.length_squared() == 0.0 {
            continue;
        }
        // #region agent log
        agent_log(
            "K",
            "client.rs:debug_input_pipeline",
            "input pipeline snapshot",
            &format!(
                r#"{{"tick":{},"player":{:?},"client_entity":{:?},"owner_entity":{:?},"owner_matches_client":{},"in_rollback":{},"action_x":{:.4},"action_y":{:.4}}}"#,
                tick,
                player,
                client_entity,
                controlled_by.owner,
                controlled_by.owner == client_entity,
                in_rollback,
                action.0.x,
                action.0.y,
            ),
        );
        // #endregion
    }
}



fn handle_interpolated_spawn(

    trigger: On<Add, Interpolated>,

    mut interpolated: Query<&mut PlayerColor>,

) {

    if let Ok(mut color) = interpolated.get_mut(trigger.entity) {

        let hsva = Hsva {

            saturation: 0.1,

            ..Hsva::from(color.0)

        };

        color.0 = Color::from(hsva);

    }

}


