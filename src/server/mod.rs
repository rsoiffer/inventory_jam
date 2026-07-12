mod player;

use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

use crate::enemy::server::spawn_enemy;
use crate::networking::SEND_INTERVAL;
use crate::projectile::server::{fire_player_projectiles, simulate_server_projectiles};
use crate::projectile::shared::projectile_collision_system;
use crate::protocol::messages::DebugServerMessage;
use crate::protocol::*;
use player::*;

pub struct ExampleServerPlugin;

impl Plugin for ExampleServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReplicationMetadata::new(SEND_INTERVAL));

        app.add_observer(handle_new_client);
        app.add_observer(handle_connected);

        app.add_systems(
            FixedUpdate,
            (
                authoritative_player_movement,
                update_player_aim_direction,
                fire_player_projectiles,
                simulate_server_projectiles,
                projectile_collision_system,
                spawn_enemy,
            )
                .chain(),
        );
        app.add_systems(Update, send_debug_server_message);
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

        //info!("Sending message: {:?}", message);

        sender
            .send::<_, ServerEventsChannel>(&message, server.into_inner(), &NetworkTarget::All)
            .unwrap_or_else(|e| {
                error!("Failed to send message: {:?}", e);
            });
    }
}
