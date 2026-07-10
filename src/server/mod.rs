//! Server gameplay systems for connection handling and authoritative simulation.

/// Player spawn and authoritative movement systems.
mod player;

use crate::net::SEND_INTERVAL;
use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

use crate::protocol::messages::DebugServerMessage;
use crate::protocol::*;
use player::*;

/// Installs server observers and authoritative gameplay systems.
pub struct ExampleServerPlugin;

impl Plugin for ExampleServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReplicationMetadata::new(SEND_INTERVAL));

        app.add_observer(handle_new_client);
        app.add_observer(handle_connected);

        app.add_systems(FixedUpdate, authoritative_player_movement);
        app.add_systems(Update, send_debug_server_message);
    }
}

/// Adds replication sender state when a new client link is created.
pub(crate) fn handle_new_client(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert((ReplicationSender, Name::from("Client")));
}

/// Sends a debug message to all clients when `M` is pressed.
pub(crate) fn send_debug_server_message(
    mut sender: ServerMultiMessageSender,
    server: Single<&Server>,
    input: Option<Res<ButtonInput<KeyCode>>>,
) {
    if input.is_some_and(|input| input.just_pressed(KeyCode::KeyM)) {
        let message = DebugServerMessage(5);

        info!("Sending message: {:?}", message);

        sender
            .send::<_, ServerEventsChannel>(&message, server.into_inner(), &NetworkTarget::All)
            .unwrap_or_else(|e| {
                error!("Failed to send message: {:?}", e);
            });
    }
}
