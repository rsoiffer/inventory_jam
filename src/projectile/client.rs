use bevy::prelude::*;
use lightyear::{core::timeline::LocalTimeline, prediction::Predicted};

use crate::{
    app::ClientState,
    projectile::protocol::{PlayerProjectile, ProjectilePosition},
    projectile::shared::{check_projectile_range, move_projectile},
    protocol::rooms::GameRoom,
    settings::GameSettings,
    shared::FixedGameplaySet,
};

pub struct ProjectileClientPlugin;

impl Plugin for ProjectileClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            simulate_predicted_projectiles
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(ClientState::Playing)),
        );
    }
}

/// Advances only locally predicted projectiles.
fn simulate_predicted_projectiles(
    mut commands: Commands,
    mut projectiles: Query<
        (
            Entity,
            &PlayerProjectile,
            &mut ProjectilePosition,
            &GameRoom,
        ),
        With<Predicted>,
    >,
    local_timeline: Res<LocalTimeline>,
    settings: Res<GameSettings>,
) {
    for (entity, projectile, mut position, room) in &mut projectiles {
        move_projectile(&mut position, projectile);
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
