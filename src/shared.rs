use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear_examples_common::shared::SharedSettings;

use crate::protocol::{PlayerAction, PlayerPosition, ProtocolPlugin};

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);
    }
}

pub const SHARED_SETTINGS: SharedSettings = SharedSettings {
    protocol_id: 0,
    private_key: [0; 32],
};

pub(crate) fn shared_movement_behaviour(
    mut position: Mut<PlayerPosition>,
    actions: &ActionState<PlayerAction>,
) {
    const MOVE_SPEED: f32 = 10.0;

    let movement = actions.clamped_axis_pair(&PlayerAction::Move);

    if movement != Vec2::ZERO {
        position.0 += movement * MOVE_SPEED;
    }

    if actions.just_pressed(&PlayerAction::Fire) {
        info!("Fire Pressed!");
    }

    if actions.just_pressed(&PlayerAction::Interact) {
        info!("Interact Pressed!");
    }

    if actions.just_pressed(&PlayerAction::UseSkill) {
        info!("UseSkill Pressed!");
    }

    let aim = actions.clamped_axis_pair(&PlayerAction::Aim);

    if aim != Vec2::ZERO {
        info!(?aim, "Aim");
    }
}
