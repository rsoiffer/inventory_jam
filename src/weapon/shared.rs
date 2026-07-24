use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;
use lightyear::{
    core::{tick::TickDuration, timeline::LocalTimeline},
    prediction::Predicted,
};

use crate::{
    app::ServerState,
    player::{PlayerAimDirection, PlayerPosition},
    projectile::ProjectileBuffer,
    protocol::inputs::PlayerAction,
    settings::GameSettings,
    weapon::protocol::{EquippedWeapon, WeaponCooldown, WeaponKind},
};

fn runs_local_sim(predicted: bool, host_server: bool, authoritative: bool) -> bool {
    !(host_server && predicted) && (authoritative || predicted)
}

pub(crate) fn fire_equipped_weapons(
    mut players: Query<(
        Has<Predicted>,
        &PlayerPosition,
        &PlayerAimDirection,
        &ActionState<PlayerAction>,
        &EquippedWeapon,
        &mut WeaponCooldown,
        &mut ProjectileBuffer,
    )>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
    app_state: Option<Res<State<ServerState>>>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let host_server = !host_server.is_empty();
    let authoritative = app_state
        .as_ref()
        .is_some_and(|state| matches!(state.get(), ServerState::Hosting));

    for (
        predicted,
        player_position,
        aim_direction,
        actions,
        equipped,
        mut cooldown,
        mut buffer,
    ) in &mut players
    {
        if !runs_local_sim(predicted, host_server, authoritative) {
            continue;
        }
        if !actions.pressed(&PlayerAction::Fire) || !cooldown.is_ready(local_timeline.tick()) {
            continue;
        }

        let direction = aim_direction.0.normalize_or_zero();
        if direction == Vec2::ZERO {
            continue;
        }

        let weapon = settings.weapons.get(equipped.kind);
        let position = player_position.0
            + direction
                * (settings.player.half_size
                    + weapon.projectile_radius.max(0.0)
                    + settings.projectile.spawn_gap);

        buffer.insert(
            equipped.kind,
            position,
            direction * weapon.projectile_speed,
            local_timeline.tick(),
        );
        cooldown.restart(
            &local_timeline,
            &tick_duration,
            weapon.attacks_per_second_at(equipped.level),
        );
    }
}

const EQUIP_ACTIONS: [(PlayerAction, WeaponKind); 6] = [
    (PlayerAction::EquipSword, WeaponKind::Sword),
    (PlayerAction::EquipSpear, WeaponKind::Spear),
    (PlayerAction::EquipStaff, WeaponKind::Staff),
    (PlayerAction::EquipBow, WeaponKind::Bow),
    (PlayerAction::EquipShuriken, WeaponKind::Shuriken),
    (PlayerAction::EquipBoomerang, WeaponKind::Boomerang),
];

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
    let host_server = !host_server.is_empty();
    let authoritative = app_state
        .as_ref()
        .is_some_and(|state| matches!(state.get(), ServerState::Hosting));

    for (predicted, actions, mut equipped, mut cooldown) in &mut players {
        if !runs_local_sim(predicted, host_server, authoritative) {
            continue;
        }

        let Some(kind) = EQUIP_ACTIONS
            .into_iter()
            .find_map(|(action, kind)| actions.just_pressed(&action).then_some(kind))
        else {
            continue;
        };
        if equipped.kind == kind {
            continue;
        }
        equipped.kind = kind;
        *cooldown = WeaponCooldown::default();
    }
}
