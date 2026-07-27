use bevy::prelude::*;
use lightyear::{
    connection::network_target::NetworkTarget,
    core::timeline::LocalTimeline,
    prediction::despawn::PredictionDespawnCommandsExt,
    prelude::{InterpolationTarget, PreSpawned, PredictionTarget, Replicate},
};

use crate::{
    projectile::protocol::{PlayerProjectile, ProjectilePosition},
    shared::ARENA_WORLD_BOUNDS,
};

pub struct SpawnProjectile {
    pub projectile: PlayerProjectile,
    pub spawn_position: Vec2,
    pub is_authoritative: bool,
    /// Determines how this entity is matched with its authoritative/predicted
    /// counterpart. Callers that fire independently on both sides from a
    /// shared deterministic schedule can rely on `PreSpawned::default()`
    /// (hashed from the spawn tick); callers that cross the network for a
    /// single asynchronous event (e.g. a client-triggered message) need an
    /// explicit hash that doesn't depend on either side's live tick, since
    /// those aren't guaranteed to line up once network latency is involved.
    pub prespawn: PreSpawned,
}

impl Command for SpawnProjectile {
    type Out = ();

    fn apply(self, world: &mut World) -> Self::Out {
        info!("Spawning projectile");
        let mut binding = world.commands();
        let mut entity = binding.spawn((
            self.projectile,
            ProjectilePosition(self.spawn_position),
            self.prespawn,
            Name::new("Projectile"),
        ));

        if self.is_authoritative {
            info!("Adding projectile authoritative components");
            entity.insert((
                Replicate::to_clients(NetworkTarget::All),
                PredictionTarget::to_clients(NetworkTarget::All),
                InterpolationTarget::to_clients(NetworkTarget::All),
            ));
        }
    }
}

pub fn move_projectile(position: &mut ProjectilePosition, projectile: &PlayerProjectile) {
    position.0 += projectile.direction.normalize_or_zero() * projectile.speed_per_tick;
}

pub fn projectile_reached_max_range(
    position: &ProjectilePosition,
    projectile: &PlayerProjectile,
) -> bool {
    position.0.distance_squared(projectile.origin) >= projectile.max_range * projectile.max_range
}

/// Simulates authoritative projectile motion and lifetime expiry.
pub fn simulate_server_projectiles(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &PlayerProjectile, &mut ProjectilePosition)>,
    local_timeline: Res<LocalTimeline>,
) {
    for (projectile_entity, projectile, mut position) in &mut projectiles {
        move_projectile(&mut position, projectile);

        if projectile.expire_time.is_expired(&local_timeline.tick())
            || projectile_reached_max_range(&position, projectile)
            || projectile_is_outside_world(position.0)
        {
            // info!("Despawning projectile: lifetime expired");
            commands.entity(projectile_entity).prediction_despawn();
        }
    }
}

fn projectile_is_outside_world(position: Vec2) -> bool {
    !ARENA_WORLD_BOUNDS.contains(position)
}
