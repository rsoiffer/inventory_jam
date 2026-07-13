mod player;

use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

use crate::app::AppState;
use crate::enemy::EnemyServerPlugin;
use crate::networking::SEND_INTERVAL;
use crate::protocol::channels::ServerEventsChannel;
use crate::protocol::messages::DebugServerMessage;
use crate::server::player::{
    apply_username_messages, authoritative_player_movement, debug_switch_rooms, handle_connected,
    update_player_aim_direction,
};
use crate::shared::FixedGameplaySet;

pub struct ExampleServerPlugin;

impl Plugin for ExampleServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReplicationMetadata::new(SEND_INTERVAL));

        app.add_observer(handle_new_client);
        app.add_observer(handle_connected);

        app.add_systems(
            FixedUpdate,
            (authoritative_player_movement, update_player_aim_direction)
                .chain()
                .in_set(FixedGameplaySet::PlayerSimulation)
                .run_if(in_state(AppState::Hosting)),
        );

        app.add_systems(
            FixedUpdate,
            debug_switch_rooms.run_if(in_state(AppState::Hosting)),
        );

        app.add_plugins(EnemyServerPlugin);
        app.add_systems(
            Update,
            (apply_username_messages, send_debug_server_message)
                .run_if(in_state(AppState::Hosting)),
        );
    }
}

pub(crate) fn handle_new_client(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert((ReplicationSender, Name::from("Client")));
}

pub(crate) fn send_debug_server_message(
    mut sender: ServerMultiMessageSender,
    server: Single<&Server>,
    input: Option<Res<ButtonInput<KeyCode>>>,
) {
    if input.is_some_and(|input| input.just_pressed(KeyCode::KeyM)) {
        let message = DebugServerMessage(5);

        sender
            .send::<_, ServerEventsChannel>(&message, server.into_inner(), &NetworkTarget::All)
            .unwrap_or_else(|error| {
                error!(?error, "Failed to send debug message");
            });
    }
}
