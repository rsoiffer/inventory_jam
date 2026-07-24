use bevy::{
    ecs::{query::With, system::Query},
    math::Vec2,
    prelude::Res,
};
use leafwing_input_manager::action_state::ActionState;
use lightyear::prediction::Predicted;

use crate::{
    player::PlayerPosition,
    protocol::{inputs::PlayerAction, rooms::GameRoom},
    settings::GameSettings,
};

pub fn player_movement(
    settings: Res<GameSettings>,
    mut player_query: Query<
        (&mut PlayerPosition, &GameRoom, &ActionState<PlayerAction>),
        With<Predicted>,
    >,
) {
    for (mut position, room, actions) in player_query.iter_mut() {
        let movement = actions.clamped_axis_pair(&PlayerAction::Move);

        if movement != Vec2::ZERO {
            position.0 += movement * settings.player.move_speed;

            let local_bounds = room
                .bounds(&settings.world)
                .inflate(-settings.player.half_size);

            position.0 = position.0.clamp(local_bounds.min, local_bounds.max);
        }
    }
}
