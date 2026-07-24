use bevy::{
    ecs::{query::With, system::Query},
    math::Vec2,
    prelude::Res,
};
use leafwing_input_manager::action_state::ActionState;
use lightyear::prediction::Predicted;

use crate::{
    player::protocol::{PlayerAimDirection, PlayerHealth, PlayerPosition},
    protocol::{inputs::PlayerAction, rooms::GameRoom},
    settings::GameSettings,
};

pub fn apply_player_movement(
    position: &mut PlayerPosition,
    room: &GameRoom,
    health: &PlayerHealth,
    actions: &ActionState<PlayerAction>,
    settings: &GameSettings,
) {
    if health.current == 0 {
        return;
    }

    let movement = actions.clamped_axis_pair(&PlayerAction::Move);

    if movement != Vec2::ZERO {
        position.0 += movement * settings.player.move_speed;

        let local_bounds = room
            .bounds(&settings.world)
            .inflate(-settings.player.half_size);

        position.0 = position.0.clamp(local_bounds.min, local_bounds.max);
    }
}

pub fn apply_player_aim(
    aim_direction: &mut PlayerAimDirection,
    actions: &ActionState<PlayerAction>,
) {
    let aim = actions.clamped_axis_pair(&PlayerAction::Aim);

    if aim.length_squared() > 0.0001 {
        aim_direction.0 = aim.normalize_or_zero();
    }
}

/// Client prediction: only the local predicted player copy.
pub fn predicted_player_movement(
    settings: Res<GameSettings>,
    mut player_query: Query<
        (
            &mut PlayerPosition,
            &GameRoom,
            &PlayerHealth,
            &ActionState<PlayerAction>,
        ),
        With<Predicted>,
    >,
) {
    for (mut position, room, health, actions) in player_query.iter_mut() {
        apply_player_movement(&mut position, room, health, actions, &settings);
    }
}

/// Client prediction: aim from the same networked ActionState the server uses.
pub fn predicted_player_aim(
    mut player_query: Query<(&ActionState<PlayerAction>, &mut PlayerAimDirection), With<Predicted>>,
) {
    for (actions, mut aim_direction) in &mut player_query {
        apply_player_aim(&mut aim_direction, actions);
    }
}
