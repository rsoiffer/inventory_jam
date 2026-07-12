use bevy::prelude::*;

use crate::projectile::{
    protocol::{PlayerProjectile, ProjectileImpact},
    shared::{
        self as projectile_shared, ClientImpactVisual, ClientProjectileVisual, ImpactPosition,
        ProjectileLifetime, ProjectilePosition, ServerImpact, ServerProjectile,
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

    // Host-client mode also contains the server-owned entity. Only the
    // replicated client copy receives local simulation and visuals.
    if server_projectile.is_some() {
        return;
    }

    commands.entity(entity).insert((
        Name::new(format!("Projectile: {}", projectile.owner)),
        ProjectilePosition(projectile.origin),
        ProjectileLifetime::from_projectile(projectile),
        ClientProjectileVisual,
    ));
}

pub(crate) fn initialize_projectile_impact(
    trigger: On<Add, ProjectileImpact>,
    mut commands: Commands,
    impacts: Query<(&ProjectileImpact, Option<&ServerImpact>)>,
) {
    let entity = trigger.entity;

    let Ok((impact, server_impact)) = impacts.get(entity) else {
        return;
    };

    if server_impact.is_some() {
        return;
    }

    commands.entity(entity).insert((
        Name::new("Projectile Impact"),
        ImpactPosition(impact.position),
        ClientImpactVisual,
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

        if lifetime.remaining_ticks == 0
            || projectile_shared::projectile_reached_max_range(*position, projectile)
        {
            // Never despawn a replicated entity locally. Remove only the local
            // simulation state and wait for the authoritative server despawn.
            commands.entity(entity).remove::<(
                ClientProjectileVisual,
                ProjectilePosition,
                ProjectileLifetime,
            )>();
        }
    }
}
