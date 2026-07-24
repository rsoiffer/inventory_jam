use bevy::prelude::*;
use lightyear::{
    connection::network_target::NetworkTarget,
    core::timeline::LocalTimeline,
    prediction::despawn::PredictionDespawnCommandsExt,
    prelude::{InterpolationTarget, PreSpawned, PredictionTarget, Replicate},
};

use crate::{
    projectile::protocol::{PlayerProjectile, ProjectilePosition},
    protocol::rooms::GameRoom,
    settings::GameSettings,
};

pub struct SpawnProjectile {
    pub projectile: PlayerProjectile,
    pub spawn_position: Vec2,
    pub room: GameRoom,
    pub is_authoritative: bool,
}

impl Command for SpawnProjectile {
    type Out = ();

    fn apply(self, world: &mut World) -> Self::Out {
        info!("Spawning projectile");
        let mut binding = world.commands();
        let mut entity = binding.spawn((
            self.projectile,
            ProjectilePosition(self.spawn_position),
            self.room,
            PreSpawned::default(),
            Name::new("Projectile"),
        ));

        if self.is_authoritative {
            let owner = self.projectile.owner;

            entity.insert((
                Replicate::to_clients(NetworkTarget::All),
                PredictionTarget::to_clients(NetworkTarget::Single(owner)),
                InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(owner)),
            ));
        }
    }
}

pub fn projectile_spawn_position(
    player_position: Vec2,
    direction: Vec2,
    projectile_radius: f32,
    settings: &GameSettings,
) -> Vec2 {
    player_position
        + direction.normalize_or_zero()
            * (settings.player.half_size
                + projectile_radius.max(0.0)
                + settings.projectile.spawn_gap)
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

pub(crate) fn check_projectile_range(
    commands: &mut Commands<'_, '_>,
    local_timeline: &Res<'_, LocalTimeline>,
    projectile_entity: Entity,
    projectile: &PlayerProjectile,
    position: &ProjectilePosition,
    room: &GameRoom,
    settings: &GameSettings,
) {
    if projectile.expire_time.is_expired(&local_timeline.tick())
        || projectile_reached_max_range(position, projectile)
        || projectile_is_outside_world(position.0, room, settings)
    {
        commands.entity(projectile_entity).prediction_despawn();
    }
}

pub(crate) fn projectile_is_outside_world(
    position: Vec2,
    room: &GameRoom,
    settings: &GameSettings,
) -> bool {
    !room.bounds(&settings.world).contains(position)
}
