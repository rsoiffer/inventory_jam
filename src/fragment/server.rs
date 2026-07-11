use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

use crate::{
    fragment::{
        protocol::Fragment,
        shared::{self as fragment_shared, FragmentLifetime, FragmentPosition, ServerFragment},
    },
    protocol::{PlayerAction, PlayerPosition},
    shared,
};

pub(crate) fn drop_fragments_at_player(
    mut commands: Commands,
    players: Query<(&PlayerPosition, &ActionState<PlayerAction>, Has<Predicted>)>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (player_position, actions, predicted) in &players {
        if is_host_server && predicted {
            continue;
        }

        if !actions.just_pressed(&PlayerAction::UseSkill) {
            continue;
        }

        let spawn_position = fragment_shared::fragment_spawn_position(player_position.0);

        info!(
            position = ?player_position.0,
            "SERVER dropping Fragment"
        );

        commands.spawn((
            Fragment {
                origin: spawn_position,
            },
            FragmentPosition(spawn_position),
            FragmentLifetime {
                remaining_ticks: fragment_shared::FRAGMENT_LIFETIME_TICKS,
            },
            ServerFragment,
            Replicate::to_clients(NetworkTarget::All),
            Name::new("Server Fragment"),
        ));
    }
}

pub(crate) fn simulate_server_fragments(
    mut commands: Commands,
    mut fragments: Query<
        (
            Entity,
            &Fragment,
            &mut FragmentPosition,
            &mut FragmentLifetime,
        ),
        With<ServerFragment>,
    >,
) {
    for (entity, fragment, position, mut lifetime) in &mut fragments {
        lifetime.remaining_ticks = lifetime.remaining_ticks.saturating_sub(1);

        if lifetime.remaining_ticks == 0 || fragment_is_outside_world(position.0) {
            commands.entity(entity).despawn();
        }
    }
}

fn fragment_is_outside_world(position: Vec2) -> bool {
    let margin = Vec2::splat(100.0);
    let limit = shared::WORLD_HALF_SIZE + margin;

    position.x.abs() > limit.x || position.y.abs() > limit.y
}
