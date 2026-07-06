use bevy::prelude::*;

use crate::protocol::PlayerPosition;

pub const FOLLOW_EPSILON: f32 = 5.;

#[derive(Default, Resource, Reflect)]
#[reflect(Resource)]
pub struct Actions {
    pub player_movement: Option<Vec2>,
}

pub fn keyboard_movement(keyboard_input: &ButtonInput<KeyCode>) -> Vec2 {
    let horizontal = f32::from(
        keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight),
    ) - f32::from(
        keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft),
    );
    let vertical = f32::from(
        keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp),
    ) - f32::from(
        keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown),
    );
    Vec2::new(horizontal, vertical)
}

pub fn read_player_movement(
    keyboard_input: &ButtonInput<KeyCode>,
    touch_input: &Touches,
    player: &PlayerPosition,
    camera: Option<(&Camera, &GlobalTransform)>,
) -> Vec2 {
    let mut movement = keyboard_movement(keyboard_input);

    if let Some(touch_position) = touch_input.first_pressed_position()
        && let Some((camera, camera_transform)) = camera
        && let Ok(touch_position) = camera.viewport_to_world_2d(camera_transform, touch_position)
    {
        let diff = touch_position - player.0;
        if diff.length() > FOLLOW_EPSILON {
            movement = diff.normalize();
        }
    }

    if movement == Vec2::ZERO {
        Vec2::ZERO
    } else {
        movement.normalize()
    }
}

pub fn movement_from_player(
    keyboard_input: &ButtonInput<KeyCode>,
    touch_input: &Touches,
    player: Option<&PlayerPosition>,
    camera: Option<(&Camera, &GlobalTransform)>,
) -> Vec2 {
    let Some(player) = player else {
        return keyboard_movement(keyboard_input).normalize_or_zero();
    };
    read_player_movement(keyboard_input, touch_input, player, camera)
}

pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Actions>().init_resource::<Actions>();
    }
}
