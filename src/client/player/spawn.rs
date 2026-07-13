use bevy::prelude::*;
use lightyear::prelude::*;

use crate::protocol::messages::*;
use crate::protocol::player::{PlayerColor, PlayerId};

pub(crate) fn handle_interpolated_spawn(
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

#[allow(dead_code)]
pub(crate) fn receive_debug_server_message(
    mut receiver: Single<&mut MessageReceiver<DebugServerMessage>>,
) {
    for _message in receiver.receive() {
        //info!("Received message: {:?}", message);
    }
}
