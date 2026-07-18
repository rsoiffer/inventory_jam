use crate::app::AppState;
use crate::player::protocol::{PlayerAimDirection, PlayerBundle, PlayerPosition};
use crate::protocol::inputs::PlayerAction;
use crate::protocol::messages::TeleportRoom;
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::shared;
use crate::shared::{FixedGameplaySet, PLAYER_HALF_SIZE};
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::connection::client::Connected;
use lightyear::connection::client_of::ClientOf;
use lightyear::connection::host::HostServer;
use lightyear::prelude::*;
use rand::Rng;

pub struct PlayerServerPlugin;

impl Plugin for PlayerServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_connected);

        app.add_systems(
            FixedUpdate,
            (authoritative_player_movement, update_player_aim_direction)
                .chain()
                .in_set(FixedGameplaySet::Player)
                .run_if(in_state(AppState::Hosting)),
        );

        app.add_systems(
            FixedUpdate,
            apply_teleport_room_commands.run_if(in_state(AppState::Hosting)),
        );
    }
}

pub(crate) fn handle_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
) {
    let Ok(client_id) = query.get(trigger.entity) else {
        return;
    };

    let client_id = client_id.0;

    let entity = commands
        .spawn((
            Name::new(format!("Player: {}", client_id)),
            PlayerBundle::new(client_id, Vec2::ZERO),
            ActionState::<PlayerAction>::default(),
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::Single(client_id)),
            InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(client_id)),
            ControlledBy {
                owner: trigger.entity,
                lifetime: Default::default(),
            },
        ))
        .id();

    info!("Create player entity {entity:?} for client {client_id:?}");
}

pub(crate) fn authoritative_player_movement(
    timeline: Res<LocalTimeline>,
    host_server: Query<(), With<HostServer>>,
    mut position_query: Query<(
        &mut PlayerPosition,
        &GameRoom,
        &ActionState<PlayerAction>,
        Has<Predicted>,
    )>,
) {
    let is_host_server = !host_server.is_empty();
    let _tick = timeline.tick();

    for (position, room, actions, predicted) in position_query.iter_mut() {
        if is_host_server && predicted {
            continue;
        }

        if actions.just_pressed(&PlayerAction::Fire) {
            //info!(tick = tick.0, "SERVER received Fire");
        }

        if actions.just_pressed(&PlayerAction::Interact) {
            //info!(tick = tick.0, "SERVER received Interact");
        }

        shared::shared_movement_behaviour(position, room, actions);
    }
}

pub(crate) fn update_player_aim_direction(
    mut players: Query<(
        &ActionState<PlayerAction>,
        &mut PlayerAimDirection,
        Has<Predicted>,
    )>,
    host_server: Query<(), With<HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (actions, mut aim_direction, predicted) in &mut players {
        if is_host_server && predicted {
            continue;
        }

        let aim = actions.clamped_axis_pair(&PlayerAction::Aim);

        if aim.length_squared() > 0.0001 {
            aim_direction.0 = aim.normalize_or_zero();
        }
    }
}

pub(crate) fn apply_teleport_room_commands(
    mut receivers: Query<(Entity, &mut MessageReceiver<TeleportRoom>), With<ClientOf>>,
    mut players: Query<(
        &ControlledBy,
        &mut GameRoom,
        &mut PlayerPosition,
        Has<Predicted>,
    )>,
    host_server: Query<(), With<HostServer>>,
) {
    let is_host_server = !host_server.is_empty();
    let mut rng = rand::rng();

    for (link_entity, mut receiver) in &mut receivers {
        for _message in receiver.receive() {
            let Some((mut room, mut position)) = players
                .iter_mut()
                .find(|(controlled_by, _, _, predicted)| {
                    controlled_by.owner == link_entity && !(is_host_server && *predicted)
                })
                .map(|(_, room, position, _)| (room, position))
            else {
                warn!("no authoritative player for TeleportRoom from {link_entity:?}");
                continue;
            };

            room.room = match room.room {
                GameRooms::Arena => GameRooms::Pit,
                GameRooms::Pit => GameRooms::Arena,
            };
            position.0 = random_position_in_room(room.room, &mut rng);
        }
    }
}

fn random_position_in_room(room: GameRooms, rng: &mut impl Rng) -> Vec2 {
    let bounds = GameRoom { room }.bounds().inflate(-PLAYER_HALF_SIZE);
    Vec2::new(
        rng.random_range(bounds.min.x..=bounds.max.x),
        rng.random_range(bounds.min.y..=bounds.max.y),
    )
}
