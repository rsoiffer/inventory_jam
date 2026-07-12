use bevy::prelude::*;
use lightyear::prelude::*;

use crate::protocol::PlayerColor;
use crate::protocol::messages::*;

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

#[allow(dead_code)]
pub(crate) fn receive_debug_server_message(
    mut receiver: Single<&mut MessageReceiver<DebugServerMessage>>,
) {
    for _message in receiver.receive() {
        //info!("Received message: {:?}", message);
    }
}
