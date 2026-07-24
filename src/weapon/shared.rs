use bevy::{
    ecs::system::{Commands, Query, Res},
    math::Vec2,
    state::state::State,
};
use leafwing_input_manager::action_state::ActionState;
use lightyear::core::{tick::TickDuration, timeline::LocalTimeline};

use crate::{
    app::ServerState,
    player::{PlayerAimDirection, PlayerId, PlayerPosition},
    projectile::{
        PlayerProjectile, ProjectileLifetime, SpawnProjectile, projectile_spawn_position,
    },
    protocol::{inputs::PlayerAction, rooms::GameRoom},
    settings::GameSettings,
    weapon::protocol::{EquippedWeapon, WeaponCooldown, WeaponKind},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponStats {
    pub damage: u32,
    pub range: f32,
    pub attacks_per_second: f32,
    pub projectile_speed_per_tick: f32,
    pub projectile_radius: f32,
}

pub fn weapon_stats(kind: WeaponKind, level: u32) -> WeaponStats {
    let level = level.max(1);
    let bonus_levels = level.saturating_sub(1);

    match kind {
        WeaponKind::Sword => WeaponStats {
            damage: 20 + bonus_levels * 4,
            range: 57.6 + bonus_levels as f32 * 1.92,
            attacks_per_second: 2.5 + bonus_levels as f32 * 0.08,
            projectile_speed_per_tick: 1.888,
            projectile_radius: 0.0,
        },
        WeaponKind::Spear => WeaponStats {
            damage: 32 + bonus_levels * 6,
            range: 83.2 + bonus_levels as f32 * 2.56,
            attacks_per_second: 1.5 + bonus_levels as f32 * 0.05,
            projectile_speed_per_tick: 3.52,
            projectile_radius: 1.92,
        },
        WeaponKind::Staff => WeaponStats {
            damage: 16 + bonus_levels * 3,
            range: 73.6 + bonus_levels as f32 * 2.24,
            attacks_per_second: 3.2 + bonus_levels as f32 * 0.1,
            projectile_speed_per_tick: 2.56,
            projectile_radius: 2.24,
        },
        WeaponKind::Bow => WeaponStats {
            damage: 28 + bonus_levels * 5,
            range: 104.0 + bonus_levels as f32 * 2.88,
            attacks_per_second: 1.8 + bonus_levels as f32 * 0.06,
            projectile_speed_per_tick: 3.84,
            projectile_radius: 0.96,
        },
        WeaponKind::Shuriken => WeaponStats {
            damage: 12 + bonus_levels * 2,
            range: 67.2 + bonus_levels as f32 * 1.92,
            attacks_per_second: 4.0 + bonus_levels as f32 * 0.12,
            projectile_speed_per_tick: 3.04,
            projectile_radius: 1.28,
        },
        WeaponKind::Boomerang => WeaponStats {
            damage: 24 + bonus_levels * 4,
            range: 80.0 + bonus_levels as f32 * 2.4,
            attacks_per_second: 1.6 + bonus_levels as f32 * 0.05,
            projectile_speed_per_tick: 2.4,
            projectile_radius: 2.24,
        },
    }
}

pub(crate) fn fire_equipped_weapons(
    mut commands: Commands,
    mut players: Query<(
        &PlayerId,
        &PlayerPosition,
        &PlayerAimDirection,
        &GameRoom,
        &ActionState<PlayerAction>,
        &EquippedWeapon,
        &mut WeaponCooldown,
    )>,
    app_state: Option<Res<State<ServerState>>>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    for (player_id, player_position, aim_direction, room, actions, equipped_weapon, mut cooldown) in
        &mut players
    {
        if !actions.pressed(&PlayerAction::Fire) || !cooldown.is_ready(local_timeline.tick()) {
            continue;
        }

        let direction = aim_direction.0.normalize_or_zero();
        if direction == Vec2::ZERO {
            continue;
        }

        let stats = weapon_stats(equipped_weapon.kind, equipped_weapon.level);
        let spawn_position = projectile_spawn_position(
            player_position.0,
            direction,
            stats.projectile_radius,
            &settings,
        );
        let expire_time = ProjectileLifetime::new(
            &local_timeline.tick(),
            stats.range,
            stats.projectile_speed_per_tick,
        );

        commands.queue(SpawnProjectile {
            projectile: PlayerProjectile {
                owner: player_id.0,
                weapon: equipped_weapon.kind,
                origin: spawn_position,
                direction,
                speed_per_tick: stats.projectile_speed_per_tick,
                damage: stats.damage,
                max_range: stats.range,
                radius: stats.projectile_radius,
                expire_time,
            },
            spawn_position,
            room: *room,
            is_authoritative: app_state
                .as_ref()
                .is_some_and(|state| matches!(state.get(), ServerState::Hosting)),
        });

        cooldown.restart(&local_timeline, &tick_duration, stats.attacks_per_second);
    }
}
