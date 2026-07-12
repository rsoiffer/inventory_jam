use bevy::prelude::*;

use crate::projectile::{
    protocol::PlayerProjectile,
    shared::{
        self as projectile_shared, ClientProjectileVisual, ProjectileLifetime, ProjectilePosition,
        ServerProjectile,
    },
};

pub(crate) fn initialize_projectile(
    trigger: On<Add, PlayerProjectile>,
    mut commands: Commands,
    projectiles: Query<(&PlayerProjectile, Option<&ServerProjectile>)>,
) {
    let entity = trigger.entity;

    let Ok((projectile, server_projectile)) = projectiles.get(entity) else {
        return;
    };

    // In host-client mode, do not let the client visual system claim the server-owned projectile.
    if server_projectile.is_some() {
        return;
    }

    commands.entity(entity).insert((
        ProjectilePosition(projectile.origin),
        ProjectileLifetime {
            remaining_ticks: projectile_shared::PROJECTILE_LIFETIME_TICKS,
        },
        ClientProjectileVisual,
    ));
}

pub(crate) fn simulate_client_projectiles(
    mut commands: Commands,
    mut projectiles: Query<
        (
            Entity,
            &PlayerProjectile,
            &mut ProjectilePosition,
            &mut ProjectileLifetime,
        ),
        (With<ClientProjectileVisual>, Without<ServerProjectile>),
    >,
) {
    for (entity, projectile, mut position, mut lifetime) in &mut projectiles {
        projectile_shared::move_projectile(&mut position, projectile);

        lifetime.remaining_ticks = lifetime.remaining_ticks.saturating_sub(1);

        if lifetime.remaining_ticks == 0 {
            // Do not despawn the replicated entity locally.
            // The server owns despawn authority.
            commands.entity(entity).remove::<(
                ClientProjectileVisual,
                ProjectilePosition,
                ProjectileLifetime,
            )>();
        }
    }
}
