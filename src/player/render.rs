use crate::app::{AppState, game_is_active};
use crate::player::protocol::{
    CachedCursorAim, PlayerAimDirection, PlayerPosition, SmoothedAimDirection,
};
use crate::player::{PlayerColor, PlayerUsername};
use crate::protocol::inputs::PlayerAction;
use bevy::app::{App, Plugin, Update};
use bevy::camera::{Camera, Camera2d};
use bevy::color::Color;
use bevy::math::{Isometry2d, StableInterpolate, Vec2, Vec3};
use bevy::prelude::{
    Commands, Component, Entity, FontSize, Gizmos, GlobalTransform, IntoScheduleConfigs, Query,
    Res, Single, Text2d, TextColor, TextFont, Time, Transform, Window, With, default, in_state,
};
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use leafwing_input_manager::input_map::InputMap;
use lightyear::prediction::Predicted;
use lightyear::prelude::Controlled;

/// How quickly should the camera snap to the desired location
const CAMERA_DECAY_RATE: f32 = 2.;
const USERNAME_LABEL_OFFSET: Vec3 = Vec3::new(0.0, 40.0, 10.0);

pub struct PlayerRenderPlugin;

impl Plugin for PlayerRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_player_boxes.run_if(game_is_active));
        app.add_systems(Update, (sync_username_labels).run_if(game_is_active));
        app.add_systems(
            Update,
            draw_local_aimstick.run_if(in_state(AppState::Playing)),
        );
        app.add_systems(
            Update,
            (update_camera, sample_cursor_aim)
                .chain()
                .run_if(in_state(AppState::Playing)),
        );
        app.add_systems(
            Update,
            (smooth_local_aim_visual)
                .chain()
                .run_if(in_state(AppState::Playing)),
        );
    }
}

#[derive(Component)]
struct UsernameLabel {
    player: Entity,
}

pub(crate) fn draw_player_boxes(
    mut gizmos: Gizmos,
    players: Query<(&PlayerPosition, &PlayerColor)>,
) {
    for (position, color) in &players {
        gizmos.rect_2d(
            Isometry2d::from_translation(position.0),
            Vec2::ONE * 50.0,
            color.0,
        );
    }
}

fn sync_username_labels(
    mut commands: Commands,
    players: Query<(Entity, &PlayerPosition, &PlayerUsername)>,
    mut labels: Query<(Entity, &UsernameLabel, &mut Transform, &mut Text2d)>,
) {
    for (player, position, username) in &players {
        if let Some((_, _, mut transform, mut text)) = labels
            .iter_mut()
            .find(|(_, label, _, _)| label.player == player)
        {
            transform.translation = position.0.extend(0.0) + USERNAME_LABEL_OFFSET;
            if text.0 != username.0 {
                text.0.clone_from(&username.0);
            }
            continue;
        }

        commands.spawn((
            UsernameLabel { player },
            Text2d::new(username.0.clone()),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Anchor::BOTTOM_CENTER,
            Transform::from_translation(position.0.extend(0.0) + USERNAME_LABEL_OFFSET),
        ));
    }

    for (label_entity, label, _, _) in &labels {
        if players.get(label.player).is_err() {
            commands.entity(label_entity).despawn();
        }
    }
}

/// Client-only system that smoothly moves the camera to the center of the Player's position.
fn update_camera(
    mut camera: Single<&mut Transform, With<Camera2d>>,
    player: Single<&PlayerPosition, With<Predicted>>,
    time: Res<Time>,
) {
    let target = player.0.extend(camera.translation.z);

    camera
        .translation
        .smooth_nudge(&target, CAMERA_DECAY_RATE, time.delta_secs());
}

fn sample_cursor_aim(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    predicted_player: Single<&PlayerPosition, With<Predicted>>,
    mut input_entity: Single<
        &mut CachedCursorAim,
        (With<Controlled>, With<InputMap<PlayerAction>>),
    >,
) {
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let (camera, camera_transform) = camera.into_inner();

    let Ok(cursor_world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position)
    else {
        return;
    };

    let direction = (cursor_world_position - predicted_player.0).normalize_or_zero();

    if direction == Vec2::ZERO {
        return;
    }

    input_entity.0 = direction;
}

fn draw_local_aimstick(
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

fn smooth_local_aim_visual(
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
