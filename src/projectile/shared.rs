use bevy::prelude::*;
use lightyear::{
    connection::network_target::NetworkTarget,
    core::timeline::LocalTimeline,
    prediction::despawn::PredictionDespawnCommandsExt,
    prelude::{InterpolationTarget, PeerId, PreSpawned, PredictionTarget, Replicate},
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
        let shot_salt = self.projectile.shot_id.owner.to_bits()
            ^ (u64::from(self.projectile.shot_id.sequence) << 32);
        let mut binding = world.commands();
        let mut entity = binding.spawn((
            self.projectile,
            ProjectilePosition(self.spawn_position),
            self.room,
            PreSpawned::default_with_salt(shot_salt),
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

pub fn move_projectile(
    position: &mut ProjectilePosition,
    projectile: &PlayerProjectile,
    delta_secs: f32,
) {
    position.0 += projectile.direction.normalize_or_zero() * projectile.speed * delta_secs;
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
    let reached_max_range =
        position.0.distance_squared(projectile.origin) >= projectile.max_range * projectile.max_range;
    let outside_world = !room.bounds(&settings.world).contains(position.0);

    if projectile.expire_time.is_expired(&local_timeline.tick())
        || reached_max_range
        || outside_world
    {
        commands.entity(projectile_entity).prediction_despawn();
    }
}

/// Pure hit test against a player target. Does not mutate health.
pub fn projectile_can_hit_player(
    projectile: &PlayerProjectile,
    projectile_position: Vec2,
    projectile_room: &GameRoom,
    player_id: PeerId,
    player_position: Vec2,
    player_room: &GameRoom,
    player_health: u32,
    player_collision_radius: f32,
) -> bool {
    if player_id == projectile.owner || player_room != projectile_room || player_health == 0 {
        return false;
    }

    let collision_radius = projectile.radius + player_collision_radius;
    projectile_position.distance_squared(player_position) <= collision_radius * collision_radius
}

/// Pure hit test against an enemy target. Does not mutate health.
pub fn projectile_can_hit_enemy(
    projectile: &PlayerProjectile,
    projectile_position: Vec2,
    projectile_room: &GameRoom,
    enemy_position: Vec2,
    enemy_room: &GameRoom,
    enemy_health: u32,
    enemy_collision_radius: f32,
) -> bool {
    if enemy_room != projectile_room || enemy_health == 0 {
        return false;
    }

    let collision_radius = projectile.radius + enemy_collision_radius;
    projectile_position.distance_squared(enemy_position) <= collision_radius * collision_radius
}
