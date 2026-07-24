use bevy::prelude::*;
use lightyear::{
    core::{tick::TickDuration, timeline::LocalTimeline},
    prediction::Predicted,
};

use crate::{
    app::ServerState,
    enemy::{EnemyHealth, EnemyKind, EnemyPosition},
    fragment::api::SpawnFragmentPool,
    player::api::AddKillsToAristeia,
    player::protocol::PlayerAristeia,
    player::PlayerId,
    projectile::protocol::{self, ProjectileBuffer},
    protocol::rooms::GameRoom,
    settings::GameSettings,
    shared::FixedGameplaySet,
    weapon::protocol::EquippedWeapon,
    world::api::AddGlobalAristeia,
};

pub struct ProjectileServerPlugin;

impl Plugin for ProjectileServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            simulate_server_projectile_buffers
                .in_set(FixedGameplaySet::Projectile)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

fn simulate_server_projectile_buffers(
    mut commands: Commands,
    mut players: Query<(
        Has<Predicted>,
        &PlayerId,
        &GameRoom,
        &mut ProjectileBuffer,
        &PlayerAristeia,
        &EquippedWeapon,
    )>,
    mut enemies: Query<(
        Entity,
        &EnemyPosition,
        &EnemyKind,
        &GameRoom,
        &mut EnemyHealth,
    )>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    mut fragment_drops: MessageWriter<SpawnFragmentPool>,
    mut aristeia_tracking: MessageWriter<AddKillsToAristeia>,
    mut global_aristeia: MessageWriter<AddGlobalAristeia>,
    settings: Res<GameSettings>,
) {
    let skip_predicted = !host_server.is_empty();
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();

    for (predicted, owner_id, room, mut buffer, aristeia, equipped) in &mut players {
        if skip_predicted && predicted {
            continue;
        }

        let owner = owner_id.0;
        let personal_aristeia = aristeia.current;
        let level = equipped.level;
        buffer.simulate(room, tick, tick_secs, level, &settings, |weapon, pos| {
            let stats = settings.weapons.get(weapon);
            let damage = stats.damage_at(level);
            let radius = stats.projectile_radius;

            for (entity, enemy, kind, enemy_room, mut health) in &mut enemies {
                if enemy_room != room
                    || health.current == 0
                    || !protocol::overlaps(
                        pos,
                        enemy.0,
                        radius,
                        kind.collision_radius(settings.enemy.collision_radius),
                    )
                {
                    continue;
                }
                health.current = health.current.saturating_sub(damage);
                if health.current == 0 {
                    let bonus = u32::from(settings.fragment.bonus_drops_per_aristeia)
                        .saturating_mul(personal_aristeia);
                    let drop_count = u32::from(settings.fragment.base_drop_count)
                        .saturating_add(bonus)
                        .min(u32::from(settings.fragment.maximum_drop_count))
                        as u16;
                    fragment_drops.write(SpawnFragmentPool::new(pos, drop_count, *room));
                    commands.entity(entity).despawn();
                    aristeia_tracking.write(AddKillsToAristeia::new(owner, 1));
                    global_aristeia.write(AddGlobalAristeia(
                        settings.global_aristeia.contribution_per_kill,
                    ));
                }
                return true;
            }

            false
        });
    }
}
