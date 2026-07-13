use crate::protocol::inputs::PlayerAction;
use crate::protocol::player::{PlayerAimDirection, PlayerPosition};
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct SmoothedAimDirection(pub Vec2);

impl Default for SmoothedAimDirection {
    fn default() -> Self {
        Self(Vec2::X)
    }
}

pub(crate) fn update_predicted_player_aim_direction(
    mut players: Query<(&ActionState<PlayerAction>, &mut PlayerAimDirection), With<Predicted>>,
) {
    for (actions, mut aim_direction) in &mut players {
        let aim = actions.clamped_axis_pair(&PlayerAction::Aim);

        if aim.length_squared() > 0.0001 {
            aim_direction.0 = aim.normalize_or_zero();
        }
    }
}

pub(crate) fn smooth_local_aim_visual(
    time: Res<Time>,
    mut players: Query<(&PlayerAimDirection, &mut SmoothedAimDirection), With<Predicted>>,
) {
    const AIM_VISUAL_SMOOTH_RATE: f32 = 35.0;

    let t = 1.0 - (-AIM_VISUAL_SMOOTH_RATE * time.delta_secs()).exp();

    for (target, mut smoothed) in &mut players {
        let target_direction = target.0.normalize_or_zero();

        if target_direction == Vec2::ZERO {
            continue;
        }

        smoothed.0 = smoothed.0.lerp(target_direction, t).normalize_or_zero();
    }
}

pub(crate) fn draw_local_aimstick(
    mut gizmos: Gizmos,
    players: Query<(&PlayerPosition, &SmoothedAimDirection), With<Predicted>>,
) {
    const AIM_STICK_LENGTH: f32 = 32.0;

    for (position, smoothed_aim) in &players {
        let direction = smoothed_aim.0.normalize_or_zero();

        if direction == Vec2::ZERO {
            continue;
        }

        let start = position.0;
        let end = start + direction * AIM_STICK_LENGTH;

        gizmos.line_2d(start, end, Color::srgb(1.0, 0.85, 0.2));
        gizmos.circle_2d(
            Isometry2d::from_translation(end),
            5.0,
            Color::srgb(1.0, 0.85, 0.2),
        );
    }
}
