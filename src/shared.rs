use crate::player::PlayerPosition;
use crate::protocol::ProtocolPlugin;
use crate::protocol::inputs::PlayerAction;
use crate::protocol::rooms::GameRoom;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub static ARENA_WORLD_BOUNDS: Rect =
    Rect::from_center_size(Vec2::new(0.0, 0.0), Vec2::new(1600.0, 1200.0));
pub static SHOP_WORLD_BOUNDS: Rect =
    Rect::from_center_size(Vec2::new(1700.0, 0.0), Vec2::new(1600.0, 1200.0));
pub const PLAYER_HALF_SIZE: f32 = 25.0;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixedGameplaySet {
    Player,
    Weapon,
    Projectile,
    Persistence,
}

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                FixedGameplaySet::Player,
                FixedGameplaySet::Weapon,
                FixedGameplaySet::Projectile,
                FixedGameplaySet::Persistence,
            )
                .chain(),
        );
    }
}

pub(crate) fn shared_movement_behaviour(
    mut position: Mut<PlayerPosition>,
    room: &GameRoom,
    actions: &ActionState<PlayerAction>,
) {
    const MOVE_SPEED: f32 = 10.0;

    let movement = actions.clamped_axis_pair(&PlayerAction::Move);

    if movement != Vec2::ZERO {
        position.0 += movement * MOVE_SPEED;

        let local_bounds = room.bounds().inflate(-PLAYER_HALF_SIZE);

        position.0 = position.0.clamp(local_bounds.min, local_bounds.max);
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
}
