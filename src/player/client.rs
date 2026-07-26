use crate::app::{ClientState, LocalUsername};
use crate::player::protocol::{
    LocalAimInput, PlayerColor, PlayerId, PlayerVisual, SmoothedAimDirection,
};
use crate::player::shared::{predicted_player_aim, predicted_player_movement};
use crate::protocol::channels::ClientEventsChannel;
use crate::protocol::inputs::PlayerAction;
use crate::protocol::messages::SetUsername;
use bevy::app::{App, FixedPreUpdate, FixedUpdate, Plugin};
use bevy::color::{Color, Hsva};
use bevy::prelude::{
    Add, Commands, IntoScheduleConfigs, Name, On, Query, Res, Vec2, With, Without, in_state,
};
use leafwing_input_manager::action_state::ActionState;
use leafwing_input_manager::input_map::InputMap;
use lightyear::input::client::InputSystems;
use lightyear::interpolation::Interpolated;
use lightyear::prediction::Predicted;
use lightyear::prelude::{Client, Connected, Controlled, ControlledBy, MessageSender};

pub struct PlayerClientPlugin;

impl Plugin for PlayerClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalAimInput>();
        app.add_systems(
            FixedPreUpdate,
            write_local_aim_to_leafwing
                .in_set(InputSystems::WriteClientInputs)
                .run_if(in_state(ClientState::Playing)),
        );
        app.add_systems(
            FixedUpdate,
            (predicted_player_aim, predicted_player_movement)
                .chain()
                .in_set(crate::shared::FixedGameplaySet::Player)
                .run_if(in_state(ClientState::Playing)),
        );

        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_controlled_spawn);
        app.add_observer(handle_interpolated_spawn);
        app.add_observer(send_local_username);
    }
}

fn write_local_aim_to_leafwing(
    local_aim: Res<LocalAimInput>,
    touch_controls: Option<Res<crate::ui::TouchControlsEnabled>>,
    mut input_entities: Query<
        &mut ActionState<PlayerAction>,
        (With<Controlled>, With<InputMap<PlayerAction>>),
    >,
) {
    if touch_controls.is_some_and(|enabled| enabled.0) {
        return;
    }

    let direction = local_aim.0.normalize_or_zero();

    if direction == Vec2::ZERO {
        return;
    }

    for mut actions in &mut input_entities {
        actions.set_axis_pair(&PlayerAction::Aim, direction);
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
            .insert((PlayerVisual, SmoothedAimDirection::default()))
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
    ));
}

fn handle_interpolated_spawn(
    trigger: On<Add, Interpolated>,
    mut commands: Commands,
    mut interpolated: Query<(&mut PlayerColor, &PlayerId)>,
) {
    let entity = trigger.entity;

    if let Ok((mut color, player_id)) = interpolated.get_mut(entity) {
        commands.entity(entity).insert((
            PlayerVisual,
            Name::new(format!("Player (Interpolated): {:?}", player_id.0)),
        ));

        let hsva = Hsva {
            saturation: 0.1,
            ..Hsva::from(color.0)
        };
        color.0 = Color::from(hsva);
    }
}
