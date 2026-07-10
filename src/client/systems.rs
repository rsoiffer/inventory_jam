use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

use crate::{
    protocol::{DebugServerMessage, PlayerAction, PlayerColor, PlayerId, PlayerPosition},
    shared,
};

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
pub(crate) fn receive_message1(mut receiver: Single<&mut MessageReceiver<DebugServerMessage>>) {
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

/// When the predicted copy of the client-owned entity is spawned, do stuff
/// - assign it a different saturation
/// - keep track of it in the Global resource
#[allow(dead_code)]
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
