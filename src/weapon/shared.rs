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
    settings::{GameSettings, WeaponKindSettings},
    weapon::protocol::{EquippedWeapon, WeaponCooldown, WeaponKind},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponStats {
    pub damage: u32,
    pub range: f32,
    pub attacks_per_second: f32,
    pub projectile_speed: f32,
    pub projectile_radius: f32,
}

pub fn weapon_stats(kind: WeaponKind, level: u32, settings: &GameSettings) -> WeaponStats {
    let level = level.max(1);
    let bonus_levels = level.saturating_sub(1);
    let base = weapon_kind_settings(kind, settings);

    WeaponStats {
        damage: base.damage + bonus_levels * base.damage_per_level,
        range: base.range + bonus_levels as f32 * base.range_per_level,
        attacks_per_second: base.attacks_per_second
            + bonus_levels as f32 * base.attacks_per_second_per_level,
        projectile_speed: base.projectile_speed,
        projectile_radius: base.projectile_radius,
    }
}

fn weapon_kind_settings(kind: WeaponKind, settings: &GameSettings) -> &WeaponKindSettings {
    match kind {
        WeaponKind::Sword => &settings.weapons.sword,
        WeaponKind::Spear => &settings.weapons.spear,
        WeaponKind::Staff => &settings.weapons.staff,
        WeaponKind::Bow => &settings.weapons.bow,
        WeaponKind::Shuriken => &settings.weapons.shuriken,
        WeaponKind::Boomerang => &settings.weapons.boomerang,
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

        let stats = weapon_stats(equipped_weapon.kind, equipped_weapon.level, &settings);
        let spawn_position = projectile_spawn_position(
            player_position.0,
            direction,
            stats.projectile_radius,
            &settings,
        );
        let expire_time = ProjectileLifetime::new(
            &local_timeline.tick(),
            stats.range,
            stats.projectile_speed,
            tick_duration.0.as_secs_f32(),
        );

        commands.queue(SpawnProjectile {
            projectile: PlayerProjectile {
                owner: player_id.0,
                weapon: equipped_weapon.kind,
                origin: spawn_position,
                direction,
                speed: stats.projectile_speed,
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
