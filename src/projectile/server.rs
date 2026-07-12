use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

use crate::{
    projectile::{
        protocol::PlayerProjectile,
        shared::{
            self as projectile_shared, ProjectileLifetime, ProjectilePosition, ServerProjectile,
        },
    },
    protocol::{PlayerAction, PlayerAimDirection, PlayerPosition},
    shared,
};

pub(crate) fn fire_player_projectiles(
    mut commands: Commands,
    players: Query<(
        &PlayerPosition,
        &PlayerAimDirection,
        &ActionState<PlayerAction>,
        Has<Predicted>,
    )>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (player_position, aim_direction, actions, predicted) in &players {
        if is_host_server && predicted {
            continue;
        }

        if !actions.just_pressed(&PlayerAction::Fire) {
            continue;
        }

        let direction = aim_direction.0.normalize_or_zero();

        if direction == Vec2::ZERO {
            warn!("Projectile skipped because aim direction was zero");
            continue;
        }

        let spawn_position =
            projectile_shared::projectile_spawn_position(player_position.0, direction);

        /* info!(
            position = ?player_position.0,
            ?direction,
            "SERVER firing projectile"
        ); */

        commands.spawn((
            PlayerProjectile {
                origin: spawn_position,
                direction,
                speed_per_tick: projectile_shared::PROJECTILE_SPEED_PER_TICK,
            },
            ProjectilePosition(spawn_position),
            ProjectileLifetime {
                remaining_ticks: projectile_shared::PROJECTILE_LIFETIME_TICKS,
            },
            ServerProjectile,
            Replicate::to_clients(NetworkTarget::All),
            Name::new("Server Projectile"),
        ));
    }
}

pub(crate) fn simulate_server_projectiles(
    mut commands: Commands,
    mut projectiles: Query<
        (
            Entity,
            &PlayerProjectile,
            &mut ProjectilePosition,
            &mut ProjectileLifetime,
        ),
        With<ServerProjectile>,
    >,
) {
    for (entity, projectile, mut position, mut lifetime) in &mut projectiles {
        projectile_shared::move_projectile(&mut position, projectile);

        lifetime.remaining_ticks = lifetime.remaining_ticks.saturating_sub(1);

        if lifetime.remaining_ticks == 0 || projectile_is_outside_world(position.0) {
            commands.entity(entity).despawn();
        }
    }
}

fn projectile_is_outside_world(position: Vec2) -> bool {
    let margin = Vec2::splat(100.0);
    let limit = shared::WORLD_HALF_SIZE + margin;

    position.x.abs() > limit.x || position.y.abs() > limit.y
}
