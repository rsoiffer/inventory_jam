use crate::app::{AppState, LocalUsername};
use crate::player::protocol::{
    CachedCursorAim, PlayerAimDirection, PlayerColor, PlayerId, PlayerPosition,
    SmoothedAimDirection,
};
use crate::protocol::channels::ClientEventsChannel;
use crate::protocol::inputs::PlayerAction;
use crate::protocol::messages::{DebugServerMessage, SetUsername};
use crate::protocol::rooms::GameRoom;
use crate::shared;
use bevy::app::{App, FixedPreUpdate, FixedUpdate, Plugin, Update};
use bevy::color::{Color, Hsva};
use bevy::prelude::{
    Add, Commands, IntoScheduleConfigs, Name, On, Query, Res, Single, With, Without, in_state,
};
use leafwing_input_manager::action_state::ActionState;
use leafwing_input_manager::input_map::InputMap;
use lightyear::interpolation::Interpolated;
use lightyear::prediction::Predicted;
use lightyear::prelude::{
    Client, Connected, Controlled, ControlledBy, InputTimeline, IsSynced, MessageReceiver,
    MessageSender,
};

pub struct PlayerClientPlugin;

impl Plugin for PlayerClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedPreUpdate,
            write_cursor_aim_to_leafwing.run_if(in_state(AppState::Playing)),
        );
        app.add_systems(
            FixedUpdate,
            (player_movement, update_predicted_player_aim_direction)
                .chain()
                .run_if(in_state(AppState::Playing)),
        );

        app.add_systems(Update, receive_message1.run_if(in_state(AppState::Playing)));

        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_controlled_spawn);
        app.add_observer(handle_interpolated_spawn);
        app.add_observer(send_local_username);
    }
}

fn write_cursor_aim_to_leafwing(
    input_entity: Single<
        (&CachedCursorAim, &mut ActionState<PlayerAction>),
        (With<Controlled>, With<InputMap<PlayerAction>>),
    >,
) {
    let (cached_aim, mut actions) = input_entity.into_inner();

    actions.set_axis_pair(&PlayerAction::Aim, cached_aim.0);
}

/// The client input only gets applied to predicted entities that we own
/// This works because we only predict the user's controlled entity.
/// If we were predicting more entities, we would have to only apply movement to the player owned one.
fn player_movement(
    synced_client: Query<(), (With<Client>, With<IsSynced<InputTimeline>>)>,
    mut player_query: Query<
        (&mut PlayerPosition, &GameRoom, &ActionState<PlayerAction>),
        With<Predicted>,
    >,
) {
    if synced_client.is_empty() {
        return;
    }

    for (position, room, actions) in player_query.iter_mut() {
        shared::shared_movement_behaviour(position, room, actions);
    }
}

fn update_predicted_player_aim_direction(
    mut players: Query<(&ActionState<PlayerAction>, &mut PlayerAimDirection), With<Predicted>>,
) {
    for (actions, mut aim_direction) in &mut players {
        let aim = actions.clamped_axis_pair(&PlayerAction::Aim);

        if aim.length_squared() > 0.0001 {
            aim_direction.0 = aim.normalize_or_zero();
        }
    }
}

fn receive_message1(mut receiver: Single<&mut MessageReceiver<DebugServerMessage>>) {
    for _message in receiver.receive() {
        //info!("Received message: {:?}", message);
    }
}

fn send_local_username(
    trigger: On<Add, Connected>,
    mut senders: Query<&mut MessageSender<SetUsername>>,
    username: Res<LocalUsername>,
) {
    if let Ok(mut sender) = senders.get_mut(trigger.entity) {
        sender.send::<ClientEventsChannel>(SetUsername {
            name: username.0.clone(),
        });
    }
}

/// When the predicted copy of the client-owned entity is spawned, do stuff
/// - assign it a different saturation
/// - keep track of it in the Global resource
///
/// Note that this will be triggered multiple times: for the locally-controlled entity,
/// but also for the remote-controlled entities that are spawned with [`Interpolated`].
/// The `With<Predicted>` filter ensures we only add the `InputMarker` once.
fn handle_predicted_spawn(
    trigger: On<Add, (PlayerId, Predicted)>,
    mut commands: Commands,
    mut predicted: Query<(&mut PlayerColor, &PlayerId), With<Predicted>>,
) {
    let entity = trigger.entity;

    if let Ok((mut color, player_id)) = predicted.get_mut(entity) {
        commands
            .entity(entity)
            .insert(SmoothedAimDirection::default())
            .insert(Name::new(format!("Player (Predicted): {:?}", player_id.0)));

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
    players: Query<(&PlayerId, Option<&ControlledBy>), Without<InputMap<PlayerAction>>>,
    clients: Query<(), With<Client>>,
) {
    let entity = trigger.entity;

    let Ok((player_id, controlled_by)) = players.get(entity) else {
        return;
    };

    if let Some(controlled_by) = controlled_by
        && clients.get(controlled_by.owner).is_err()
    {
        return;
    }

    //info!("Adding Leafwing InputMap to controlled player {entity:?} {player_id:?}");

    commands.entity(entity).insert((
        Name::new(format!("Player (Controlled): {}", player_id.0)),
        PlayerAction::default_input_map(),
        CachedCursorAim::default(),
    ));
}

fn handle_interpolated_spawn(
    trigger: On<Add, Interpolated>,
    mut commands: Commands,
    mut interpolated: Query<(&mut PlayerColor, &PlayerId)>,
) {
    let entity = trigger.entity;

    if let Ok((mut color, player_id)) = interpolated.get_mut(entity) {
        commands.entity(entity).insert(Name::new(format!(
            "Player (Interpolated): {:?}",
            player_id.0
        )));

        let hsva = Hsva {
            saturation: 0.1,
            ..Hsva::from(color.0)
        };
        color.0 = Color::from(hsva);
    }
}
