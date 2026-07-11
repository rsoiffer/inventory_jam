use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::connection::client::Connected;
use lightyear::connection::client_of::ClientOf;
use lightyear::connection::host::HostServer;
use lightyear::prelude::*;

use crate::protocol::player::PlayerBundle;
use crate::protocol::{PlayerAction, PlayerAimDirection, PlayerPosition};
use crate::shared;

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

    info!(
        "Create player entity {:?} for client {:?}",
        entity, client_id
    );
}

pub(crate) fn authoritative_player_movement(
    timeline: Res<LocalTimeline>,
    host_server: Query<(), With<HostServer>>,
    mut position_query: Query<(
        &mut PlayerPosition,
        &ActionState<PlayerAction>,
        Has<Predicted>,
    )>,
) {
    let is_host_server = !host_server.is_empty();
    let _tick = timeline.tick();

    for (position, actions, predicted) in position_query.iter_mut() {
        if is_host_server && predicted {
            continue;
        }

        if actions.just_pressed(&PlayerAction::Fire) {
            //info!(tick = tick.0, "SERVER received Fire");
        }

        if actions.just_pressed(&PlayerAction::Interact) {
            //info!(tick = tick.0, "SERVER received Interact");
        }

        shared::shared_movement_behaviour(position, actions);
    }
}

pub(crate) fn update_player_aim_direction(
    mut players: Query<(
        &ActionState<PlayerAction>,
        &mut PlayerAimDirection,
        Has<Predicted>,
    )>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (actions, mut aim_direction, predicted) in &mut players {
        if is_host_server && predicted {
            continue;
        }

        let aim = actions.clamped_axis_pair(&PlayerAction::Aim);

        /* info!(
            ?aim,
            current = ?aim_direction.0,
            "SERVER inspected aim before projectile"
        ); */

        if aim.length_squared() > 0.0001 {
            aim_direction.0 = aim.normalize_or_zero();
        }
    }
}
