use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;

use crate::enemy::{EnemyHealth, EnemyPosition};
use crate::persistence::CachedPersistentState;
use crate::protocol::inputs::PlayerAction;
use crate::protocol::rooms::GameRoom;
use crate::settings::GameSettings;

pub(crate) fn aim_and_fire_nearest_enemy(
    actions: &mut ActionState<PlayerAction>,
    player_position: Vec2,
    player_room: &GameRoom,
    health_current: u32,
    cache: &CachedPersistentState,
    settings: &GameSettings,
    enemies: &Query<(&EnemyPosition, &EnemyHealth, &GameRoom)>,
) {
    if health_current == 0 {
        actions.release(&PlayerAction::Fire);
        return;
    }

    let weapon_id = if settings.weapons.get(cache.equipped_weapon_id).is_some() {
        cache.equipped_weapon_id
    } else {
        settings.weapons.default_id()
    };
    let Some(weapon) = settings.weapons.get(weapon_id) else {
        actions.release(&PlayerAction::Fire);
        return;
    };

    let range_squared = weapon.range * weapon.range;
    let mut nearest = None::<(f32, Vec2)>;

    for (enemy_position, enemy_health, enemy_room) in enemies {
        if enemy_health.current == 0 || enemy_room != player_room {
            continue;
        }

        let offset = enemy_position.0 - player_position;
        let distance_squared = offset.length_squared();
        if distance_squared > range_squared || distance_squared <= f32::EPSILON {
            continue;
        }

        if nearest.is_none_or(|(best, _)| distance_squared < best) {
            nearest = Some((distance_squared, offset));
        }
    }

    if let Some((_, offset)) = nearest {
        actions.set_axis_pair(&PlayerAction::Aim, offset.normalize_or_zero());
        actions.press(&PlayerAction::Fire);
    } else {
        actions.release(&PlayerAction::Fire);
    }
}
