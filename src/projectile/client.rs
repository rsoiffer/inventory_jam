use bevy::prelude::*;
use lightyear::{
    core::timeline::LocalTimeline,
    prediction::Predicted,
    prelude::PreSpawned,
};

use crate::{
    app::ClientState,
    enemy::{EnemyHealth, EnemyKind, EnemyPosition},
    player::{PlayerId, PlayerPosition},
    player::protocol::PlayerHealth,
    projectile::protocol::{
        PlayerProjectile, PredictedHitHidden, PredictedProjectileImpact, ProjectileImpact,
        ProjectilePosition,
    },
    projectile::shared::{
        check_projectile_range, move_projectile, projectile_can_hit_enemy, projectile_can_hit_player,
    },
    protocol::rooms::GameRoom,
    settings::GameSettings,
    shared::FixedGameplaySet,
};

/// Local projectiles before server match (`PreSpawned`) and after (`Predicted`).
type ClientSimulatedProjectile = Or<(With<Predicted>, With<PreSpawned>)>;

pub struct ProjectileClientPlugin;

impl Plugin for ProjectileClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                simulate_predicted_projectiles,
                predict_projectile_impacts,
                expire_predicted_impacts,
            )
                .chain()
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(ClientState::Playing)),
        );
    }
}

/// Advances locally predicted and still-unmatched pre-spawned projectiles.
fn simulate_predicted_projectiles(
    mut commands: Commands,
    mut projectiles: Query<
        (
            Entity,
            &PlayerProjectile,
            &mut ProjectilePosition,
            &GameRoom,
        ),
        ClientSimulatedProjectile,
    >,
    local_timeline: Res<LocalTimeline>,
    settings: Res<GameSettings>,
    time: Res<Time>,
) {
    let delta_secs = time.delta_secs();
    for (entity, projectile, mut position, room) in &mut projectiles {
        move_projectile(&mut position, projectile, delta_secs);
        check_projectile_range(
            &mut commands,
            &local_timeline,
            entity,
            projectile,
            &position,
            room,
            &settings,
        );
    }
}

/// Cosmetic-only local hit feedback. Does not mutate health or despawn the
/// predicted projectile simulation entity.
fn predict_projectile_impacts(
    mut commands: Commands,
    projectiles: Query<
        (
            Entity,
            &PlayerProjectile,
            &ProjectilePosition,
            &GameRoom,
        ),
        (ClientSimulatedProjectile, Without<PredictedHitHidden>),
    >,
    players: Query<(
        &PlayerId,
        &PlayerPosition,
        &GameRoom,
        &PlayerHealth,
    )>,
    enemies: Query<(&EnemyPosition, &EnemyKind, &GameRoom, &EnemyHealth)>,
    predicted_impacts: Query<&PredictedProjectileImpact>,
    confirmed_impacts: Query<&ProjectileImpact>,
    settings: Res<GameSettings>,
) {
    for (entity, projectile, position, room) in &projectiles {
        if predicted_impacts
            .iter()
            .any(|impact| impact.shot_id == projectile.shot_id)
            || confirmed_impacts
                .iter()
                .any(|impact| impact.shot_id == projectile.shot_id)
        {
            continue;
        }

        let hit_player = players.iter().any(
            |(player_id, player_position, player_room, health)| {
                projectile_can_hit_player(
                    projectile,
                    position.0,
                    room,
                    player_id.0,
                    player_position.0,
                    player_room,
                    health.current,
                    settings.player.collision_radius,
                )
            },
        );

        let hit_enemy = enemies.iter().any(|(enemy_position, kind, enemy_room, health)| {
            projectile_can_hit_enemy(
                projectile,
                position.0,
                room,
                enemy_position.0,
                enemy_room,
                health.current,
                kind.collision_radius(settings.enemy.collision_radius),
            )
        });

        if !hit_player && !hit_enemy {
            continue;
        }

        commands.spawn((
            PredictedProjectileImpact {
                shot_id: projectile.shot_id,
                position: position.0,
                direction: projectile.direction,
                weapon: projectile.weapon,
                remaining_ticks: settings.projectile.predicted_impact_max_lifetime_ticks,
            },
            Name::new("Predicted Projectile Impact"),
        ));
        commands.entity(entity).insert(PredictedHitHidden);
    }
}

fn expire_predicted_impacts(
    mut commands: Commands,
    mut impacts: Query<(Entity, &mut PredictedProjectileImpact)>,
    hidden_projectiles: Query<(Entity, &PlayerProjectile), With<PredictedHitHidden>>,
) {
    for (entity, mut impact) in &mut impacts {
        impact.remaining_ticks = impact.remaining_ticks.saturating_sub(1);
        if impact.remaining_ticks > 0 {
            continue;
        }

        for (projectile_entity, projectile) in &hidden_projectiles {
            if projectile.shot_id == impact.shot_id {
                commands
                    .entity(projectile_entity)
                    .remove::<PredictedHitHidden>();
            }
        }

        commands.entity(entity).despawn();
    }
}
