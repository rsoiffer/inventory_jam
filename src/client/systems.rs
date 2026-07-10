use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

use crate::{protocol::{
    DebugServerMessage, PlayerAction, PlayerColor, PlayerId, PlayerPosition,
}, shared};

/// The client input only gets applied to predicted entities that we own
/// This works because we only predict the user's controlled entity.
/// If we were predicting more entities, we would have to only apply movement to the player owned one.
pub(crate) fn player_movement(
    synced_client: Query<(), (With<Client>, With<IsSynced<InputTimeline>>)>,
    mut position_query: Query<(&mut PlayerPosition, &ActionState<PlayerAction>), With<Predicted>>,
) {
    if synced_client.is_empty() {
        return;
    }

    for (position, actions) in position_query.iter_mut() {
        shared::shared_movement_behaviour(position, actions);
    }
}

/// System to receive messages on the client
pub(crate) fn receive_message1(
    mut receiver: Single<&mut MessageReceiver<DebugServerMessage>>,
) {
    for message in receiver.receive() {
        info!("Received message: {:?}", message);
    }
}

/// When the predicted copy of the client-owned entity is spawned, do stuff
/// - assign it a different saturation
/// - keep track of it in the Global resource
///
/// Note that this will be triggered multiple times: for the locally-controlled entity,
/// but also for the remote-controlled entities that are spawned with [`Interpolated`].
/// The `With<Predicted>` filter ensures we only add the `InputMarker` once.
pub(crate) fn handle_predicted_spawn(
    trigger: On<Add, (PlayerId, Predicted)>,
    mut predicted: Query<&mut PlayerColor, With<Predicted>>,
) {
    let entity = trigger.entity;
    if let Ok(mut color) = predicted.get_mut(entity) {
        let hsva = Hsva {
            saturation: 0.4,
            ..Hsva::from(color.0)
        };
        color.0 = Color::from(hsva);
    }
}

pub(crate) fn handle_controlled_spawn(
    trigger: On<Add, Controlled>,
    mut commands: Commands,
    players: Query<(&PlayerId, Option<&ControlledBy>), Without<InputMap<PlayerAction>>>,
    clients: Query<(), With<Client>>,
) {
    let entity = trigger.entity;

    let Ok((player_id, controlled_by)) = players.get(entity) else {
        return;
    };

    if let Some(controlled_by) = controlled_by {
        if clients.get(controlled_by.owner).is_err() {
            return;
        }
    }

    info!("Adding Leafwing InputMap to controlled player {entity:?} {player_id:?}");

    commands
        .entity(entity)
        .insert(PlayerAction::default_input_map());
}

/// When the predicted copy of the client-owned entity is spawned, do stuff
/// - assign it a different saturation
/// - keep track of it in the Global resource
pub(crate) fn handle_interpolated_spawn(
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