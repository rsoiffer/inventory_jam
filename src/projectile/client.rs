use bevy::prelude::*;
use lightyear::{
    core::{tick::TickDuration, timeline::LocalTimeline},
    prediction::Predicted,
};

use crate::{
    app::ClientState,
    enemy::{EnemyHealth, EnemyKind, EnemyPosition},
    projectile::protocol::{self, ProjectileBuffer},
    protocol::rooms::GameRoom,
    settings::GameSettings,
    shared::FixedGameplaySet,
};

pub struct ProjectileClientPlugin;

impl Plugin for ProjectileClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            simulate_predicted_projectile_buffers
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(ClientState::Playing)),
        );
    }
}

fn simulate_predicted_projectile_buffers(
    mut players: Query<(&mut ProjectileBuffer, &GameRoom), With<Predicted>>,
    enemies: Query<(&EnemyPosition, &EnemyKind, &GameRoom, &EnemyHealth)>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();

    for (mut buffer, room) in &mut players {
        buffer.simulate(room, tick, tick_secs, &settings, |weapon, pos| {
            let Some(radius) = settings
                .weapons
                .get(weapon)
                .map(|stats| stats.projectile_radius)
            else {
                return false;
            };
            enemies.iter().any(|(enemy, kind, enemy_room, health)| {
                enemy_room == room
                    && health.current > 0
                    && protocol::overlaps(
                        pos,
                        enemy.0,
                        radius,
                        kind.collision_radius(settings.enemy.collision_radius),
                    )
            })
        });
    }
}
