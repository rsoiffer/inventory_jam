use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;
use lightyear::{
    core::{tick::TickDuration, timeline::LocalTimeline},
    prediction::Predicted,
};

use crate::{
    app::ServerState,
    player::{PlayerAimDirection, PlayerId, PlayerPosition},
    projectile::{
        PlayerProjectile, ProjectileLifetime, ShotId, SpawnProjectile, projectile_spawn_position,
    },
    protocol::{inputs::PlayerAction, rooms::GameRoom},
    settings::GameSettings,
    weapon::protocol::{EquippedWeapon, ShotSequence, WeaponCooldown, WeaponKind},
};

pub(crate) fn fire_equipped_weapons(
    mut commands: Commands,
    mut players: Query<(
        Has<Predicted>,
        &PlayerId,
        &PlayerPosition,
        &PlayerAimDirection,
        &GameRoom,
        &ActionState<PlayerAction>,
        &EquippedWeapon,
        &mut WeaponCooldown,
        &mut ShotSequence,
    )>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
    app_state: Option<Res<State<ServerState>>>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let is_host_server = !host_server.is_empty();
    let is_authoritative = app_state
        .as_ref()
        .is_some_and(|state| matches!(state.get(), ServerState::Hosting));

    for (
        predicted,
        player_id,
        player_position,
        aim_direction,
        room,
        actions,
        equipped_weapon,
        mut cooldown,
        mut shot_sequence,
    ) in &mut players
    {
        if is_host_server && predicted {
            continue;
        }

        if !is_authoritative && !predicted {
            continue;
        }

        if !actions.pressed(&PlayerAction::Fire) || !cooldown.is_ready(local_timeline.tick()) {
            continue;
        }

        let direction = aim_direction.0.normalize_or_zero();
        if direction == Vec2::ZERO {
            continue;
        }

        let level = equipped_weapon.level.max(1);
        let bonus_levels = level.saturating_sub(1);
        let base = match equipped_weapon.kind {
            WeaponKind::Sword => &settings.weapons.sword,
            WeaponKind::Spear => &settings.weapons.spear,
            WeaponKind::Staff => &settings.weapons.staff,
            WeaponKind::Bow => &settings.weapons.bow,
            WeaponKind::Shuriken => &settings.weapons.shuriken,
            WeaponKind::Boomerang => &settings.weapons.boomerang,
        };
        let damage = base.damage + bonus_levels * base.damage_per_level;
        let range = base.range + bonus_levels as f32 * base.range_per_level;
        let attacks_per_second =
            base.attacks_per_second + bonus_levels as f32 * base.attacks_per_second_per_level;
        let spawn_position = projectile_spawn_position(
            player_position.0,
            direction,
            base.projectile_radius,
            &settings,
        );
        shot_sequence.0 = shot_sequence.0.wrapping_add(1);
        let shot_id = ShotId {
            owner: player_id.0,
            sequence: shot_sequence.0,
        };
        let expire_time = ProjectileLifetime::new(
            &local_timeline.tick(),
            range,
            base.projectile_speed,
            tick_duration.0.as_secs_f32(),
        );

        commands.queue(SpawnProjectile {
            projectile: PlayerProjectile {
                shot_id,
                owner: player_id.0,
                weapon: equipped_weapon.kind,
                origin: spawn_position,
                direction,
                speed: base.projectile_speed,
                damage,
                max_range: range,
                radius: base.projectile_radius,
                expire_time,
            },
            spawn_position,
            room: *room,
            is_authoritative,
        });

        cooldown.restart(&local_timeline, &tick_duration, attacks_per_second);
    }
}

pub(crate) fn switch_equipped_weapons(
    mut players: Query<(
        Has<Predicted>,
        &ActionState<PlayerAction>,
        &mut EquippedWeapon,
        &mut WeaponCooldown,
    )>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
    app_state: Option<Res<State<ServerState>>>,
) {
    let is_host_server = !host_server.is_empty();
    let is_authoritative = app_state
        .as_ref()
        .is_some_and(|state| matches!(state.get(), ServerState::Hosting));

    for (predicted, actions, mut equipped, mut cooldown) in &mut players {
        if is_host_server && predicted {
            continue;
        }

        if !is_authoritative && !predicted {
            continue;
        }

        let requested = if actions.just_pressed(&PlayerAction::EquipSword) {
            Some(WeaponKind::Sword)
        } else if actions.just_pressed(&PlayerAction::EquipSpear) {
            Some(WeaponKind::Spear)
        } else if actions.just_pressed(&PlayerAction::EquipStaff) {
            Some(WeaponKind::Staff)
        } else if actions.just_pressed(&PlayerAction::EquipBow) {
            Some(WeaponKind::Bow)
        } else if actions.just_pressed(&PlayerAction::EquipShuriken) {
            Some(WeaponKind::Shuriken)
        } else if actions.just_pressed(&PlayerAction::EquipBoomerang) {
            Some(WeaponKind::Boomerang)
        } else {
            None
        };

        let Some(kind) = requested else {
            continue;
        };

        if equipped.kind == kind {
            continue;
        }

        equipped.kind = kind;
        *cooldown = WeaponCooldown::default();
    }
}
