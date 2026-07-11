use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear_examples_common::shared::SharedSettings;

use crate::protocol::{PlayerAction, PlayerPosition, ProtocolPlugin};

pub const WORLD_HALF_SIZE: Vec2 = Vec2::new(800.0, 600.0);
pub const PLAYER_HALF_SIZE: f32 = 25.0;

pub const PROJECTILE_SPEED_PER_TICK: f32 = 20.0;
pub const PROJECTILE_LIFETIME_TICKS: u16 = 90;
pub const PROJECTILE_RADIUS: f32 = 8.0;
pub const PROJECTILE_SPAWN_OFFSET: f32 = PLAYER_HALF_SIZE + PROJECTILE_RADIUS + 2.0;

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

        let limit = WORLD_HALF_SIZE - Vec2::splat(PLAYER_HALF_SIZE);
        position.0 = position.0.clamp(-limit, limit);
    }

    if actions.just_pressed(&PlayerAction::Fire) {
        //info!("Fire Pressed!");
    }

    if actions.just_pressed(&PlayerAction::Interact) {
        //info!("Interact Pressed!");
    }

    if actions.just_pressed(&PlayerAction::UseSkill) {
        //info!("UseSkill Pressed!");
    }

    let aim = actions.axis_pair(&PlayerAction::Aim);

    if aim.length_squared() > f32::EPSILON {
        let _aim_direction = aim.normalize();

        //info!(?aim, ?aim_direction, "Aim");
    }
}
