use bevy::prelude::*;
use lightyear::{
    core::{tick::TickDuration, timeline::LocalTimeline},
    prediction::Predicted,
};

use crate::{
    app::ClientState,
    enemy::{EnemyHealth, EnemyKind, EnemyPosition},
    player::protocol::PlayerHealth,
    player::{PlayerId, PlayerPosition},
    projectile::protocol::{self, ProjectileBuffer},
    protocol::rooms::GameRoom,
    settings::GameSettings,
    shared::FixedGameplaySet,
    weapon::protocol::EquippedWeapon,
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
    mut players: Query<
        (&PlayerId, &mut ProjectileBuffer, &GameRoom, &EquippedWeapon),
        With<Predicted>,
    >,
    targets: Query<(&PlayerId, &PlayerPosition, &GameRoom, &PlayerHealth)>,
    enemies: Query<(&EnemyPosition, &EnemyKind, &GameRoom, &EnemyHealth)>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();

    for (owner_id, mut buffer, room, equipped) in &mut players {
        let owner = owner_id.0;
        buffer.simulate(room, tick, tick_secs, equipped.level, &settings, |weapon, pos| {
            let radius = settings.weapons.get(weapon).projectile_radius;
            targets.iter().any(|(id, target, target_room, health)| {
                id.0 != owner
                    && protocol::overlaps(
                        pos,
                        target.0,
                        radius,
                        settings.player.collision_radius,
                    )
                    && target_room == room
                    && health.current > 0
            }) || enemies.iter().any(|(enemy, kind, enemy_room, health)| {
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
