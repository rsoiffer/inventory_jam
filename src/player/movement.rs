use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::protocol::{PlayerAction, PlayerPosition};

pub(crate) fn shared_player_movement(
    mut position: Mut<PlayerPosition>,
    actions: &ActionState<PlayerAction>,
) {
    const MOVE_PER_TICK: f32 = 10.0;

    let movement = actions.clamped_axis_pair(&PlayerAction::Move);

    if movement != Vec2::ZERO {
        position.0 += movement * MOVE_PER_TICK;
    }

    if actions.just_pressed(&PlayerAction::Fire) {
        info!("Fire");
    }

    if actions.just_pressed(&PlayerAction::Interact) {
        info!("Interact");
    }

    if actions.just_pressed(&PlayerAction::UseSkill) {
        info!("UseSkill");
    }

    let aim = actions.clamped_axis_pair(&PlayerAction::Aim);

    if aim != Vec2::ZERO {
        trace!(?aim, "Aim input");
    }
}