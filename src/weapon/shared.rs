use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;
use lightyear::{
    core::{tick::TickDuration, timeline::LocalTimeline},
    prediction::Predicted,
};

use crate::{
    app::ServerState,
    persistence::CachedPersistentState,
    player::protocol::PlayerHealth,
    player::{PlayerAimDirection, PlayerPosition},
    projectile::ProjectileBuffer,
    protocol::inputs::PlayerAction,
    settings::GameSettings,
    weapon::protocol::WeaponCooldown,
};

pub(crate) fn fire_equipped_weapons(
    mut players: Query<(
        Has<Predicted>,
        &PlayerPosition,
        &PlayerAimDirection,
        &PlayerHealth,
        &ActionState<PlayerAction>,
        &CachedPersistentState,
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
        health,
        actions,
        cache,
        mut cooldown,
        mut buffer,
    ) in &mut players
    {
        if (host_server && predicted) || !(authoritative || predicted) {
            continue;
        }
        if health.current == 0
            || !actions.pressed(&PlayerAction::Fire)
            || !cooldown.is_ready(local_timeline.tick())
        {
            continue;
        }

        let direction = aim_direction.0.normalize_or_zero();
        if direction == Vec2::ZERO {
            continue;
        }

        let weapon_id = if settings.weapons.get(cache.equipped_weapon_id).is_some() {
            cache.equipped_weapon_id
        } else {
            settings.weapons.default_id()
        };
        let Some(weapon) = settings.weapons.get(weapon_id) else {
            continue;
        };
        let position = player_position.0
            + direction
                * (settings.player.half_size
                    + weapon.projectile_radius.max(0.0)
                    + settings.projectile.spawn_gap);

        buffer.insert(
            weapon_id,
            position,
            direction * weapon.projectile_speed,
            local_timeline.tick(),
        );
        cooldown.restart(&local_timeline, &tick_duration, weapon.attacks_per_second);
    }
}
