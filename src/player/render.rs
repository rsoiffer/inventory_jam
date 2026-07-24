use crate::app::{ClientState, game_is_active};
use crate::player::protocol::{
    LocalAimInput, PlayerAimDirection, PlayerAristeia, PlayerHealth, PlayerPosition, PlayerVisual,
    SmoothedAimDirection,
};
use crate::player::{PlayerColor, PlayerUsername};
use crate::settings::GameSettings;
use crate::world::GlobalAristeia;

use bevy::app::{App, Plugin, Update};
use bevy::camera::{Camera, Camera2d};
use bevy::color::Color;
use bevy::ecs::query::Without;
use bevy::math::{Isometry2d, StableInterpolate, Vec2};
use bevy::prelude::{
    BackgroundColor, Commands, Component, Entity, FlexDirection, FontSize, Gizmos, GlobalTransform,
    IntoScheduleConfigs, Node, OnEnter, OnExit, PositionType, Query, Res, ResMut, Single, Text,
    Text2d, TextColor, TextFont, Time, Transform, UiRect, Val, Window, With, default, in_state,
};
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

use lightyear::prediction::Predicted;

pub struct PlayerRenderPlugin;

impl Plugin for PlayerRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ClientState::Playing), spawn_aristeia_ui);

        app.add_systems(OnExit(ClientState::Playing), despawn_aristeia_ui);

        app.add_systems(
            Update,
            (draw_player_boxes, sync_username_labels).run_if(game_is_active),
        );

        app.add_systems(
            Update,
            (draw_local_aimstick, update_aristeia_ui).run_if(in_state(ClientState::Playing)),
        );

        app.add_systems(
            Update,
            (update_camera, sample_cursor_aim)
                .chain()
                .run_if(in_state(ClientState::Playing)),
        );

        app.add_systems(
            Update,
            smooth_local_aim_visual.run_if(in_state(ClientState::Playing)),
        );
    }
}

#[derive(Component)]
struct UsernameLabel {
    player: Entity,
}

#[derive(Component)]
struct AristeiaUiRoot;

#[derive(Component)]
struct AristeiaValueText;

#[derive(Component)]
struct AristeiaBarFill;

#[derive(Component)]
struct GlobalAristeiaValueText;

#[derive(Component)]
struct GlobalAristeiaBarFill;

pub(crate) fn draw_player_boxes(
    settings: Res<GameSettings>,
    mut gizmos: Gizmos,
    players: Query<(&PlayerPosition, &PlayerColor, &PlayerHealth), With<PlayerVisual>>,
) {
    for (position, color, health) in &players {
        gizmos.rect_2d(
            Isometry2d::from_translation(position.0),
            Vec2::ONE * settings.player.half_size * 2.0,
            color.0,
        );

        draw_health_bar(&settings, &mut gizmos, position, health);
    }
}

fn sync_username_labels(
    settings: Res<GameSettings>,
    mut commands: Commands,
    players: Query<(Entity, &PlayerPosition, &PlayerUsername), With<PlayerVisual>>,
    mut labels: Query<(Entity, &UsernameLabel, &mut Transform, &mut Text2d)>,
) {
    for (player, position, username) in &players {
        let existing_label = labels
            .iter_mut()
            .find(|(_, label, _, _)| label.player == player);

        if let Some((_, _, mut transform, mut text)) = existing_label {
            transform.translation = position.0.extend(0.0) + settings.player.username_label_offset;

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
            Transform::from_translation(
                position.0.extend(0.0) + settings.player.username_label_offset,
            ),
        ));
    }

    for (label_entity, label, _, _) in &labels {
        if players.get(label.player).is_err() {
            commands.entity(label_entity).despawn();
        }
    }
}

/// Client-only system that smoothly moves the camera to the local predicted
/// player's position.
fn update_camera(
    mut camera: Single<&mut Transform, With<Camera2d>>,
    player: Single<&PlayerPosition, With<Predicted>>,
    time: Res<Time>,
    settings: Res<GameSettings>,
) {
    let target = player.0.extend(camera.translation.z);

    camera.translation.smooth_nudge(
        &target,
        settings.player.camera_decay_rate,
        time.delta_secs(),
    );
}

fn sample_cursor_aim(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    predicted_player: Single<&PlayerPosition, With<Predicted>>,
    mut local_aim: ResMut<LocalAimInput>,
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

    local_aim.0 = direction;
}

fn draw_local_aimstick(
    settings: Res<GameSettings>,
    mut gizmos: Gizmos,
    players: Query<(&PlayerPosition, &SmoothedAimDirection), With<Predicted>>,
) {
    for (position, smoothed_aim) in &players {
        let direction = smoothed_aim.0.normalize_or_zero();

        if direction == Vec2::ZERO {
            continue;
        }

        let start = position.0;
        let end = start + direction * settings.player.aim_stick_length;

        gizmos.line_2d(start, end, Color::srgb(1.0, 0.85, 0.2));

        gizmos.circle_2d(
            Isometry2d::from_translation(end),
            5.0,
            Color::srgb(1.0, 0.85, 0.2),
        );
    }
}

fn smooth_local_aim_visual(
    settings: Res<GameSettings>,
    time: Res<Time>,
    mut players: Query<(&PlayerAimDirection, &mut SmoothedAimDirection), With<Predicted>>,
) {
    let interpolation = 1.0 - (-settings.player.aim_visual_smooth_rate * time.delta_secs()).exp();

    for (target, mut smoothed) in &mut players {
        let target_direction = target.0.normalize_or_zero();

        if target_direction == Vec2::ZERO {
            continue;
        }

        smoothed.0 = smoothed
            .0
            .lerp(target_direction, interpolation)
            .normalize_or_zero();
    }
}

fn draw_health_bar(
    settings: &GameSettings,
    gizmos: &mut Gizmos<'_, '_>,
    position: &PlayerPosition,
    health: &PlayerHealth,
) {
    let health_fraction = if health.maximum == 0 {
        0.0
    } else {
        (health.current as f32 / health.maximum as f32).clamp(0.0, 1.0)
    };

    let background_center = position.0 + Vec2::new(0.0, settings.player.health_bar_offset_y);

    gizmos.rect_2d(
        Isometry2d::from_translation(background_center),
        Vec2::new(
            settings.player.health_bar_width,
            settings.player.health_bar_height,
        ),
        Color::srgb(0.2, 0.05, 0.05),
    );

    if health_fraction <= 0.0 {
        return;
    }

    let foreground_size = Vec2::new(
        settings.player.health_bar_width * health_fraction,
        settings.player.health_bar_height,
    );

    let foreground_center = position.0
        + Vec2::new(
            (foreground_size.x - settings.player.health_bar_width) * 0.5,
            settings.player.health_bar_offset_y,
        );

    gizmos.rect_2d(
        Isometry2d::from_translation(foreground_center),
        foreground_size,
        Color::srgb(0.9, 0.2, 0.2),
    );
}

fn spawn_aristeia_ui(
    mut commands: Commands,
    settings: Res<GameSettings>,
    existing_ui: Query<Entity, With<AristeiaUiRoot>>,
) {
    // Prevent duplicate UI roots if the state is entered more than once
    // without a complete cleanup.
    if !existing_ui.is_empty() {
        return;
    }

    commands
        .spawn((
            AristeiaUiRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Px(24.0),
                width: Val::Px(settings.player.aristeia_bar_width),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.04, 0.82)),
        ))
        .with_children(|root| {
            root.spawn((
                AristeiaValueText,
                Text::new("ARISTEIA 0"),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(settings.player.aristeia_bar_height),
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.12, 0.12, 0.14)),
            ))
            .with_children(|bar_background| {
                bar_background.spawn((
                    AristeiaBarFill,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.95, 0.64, 0.12)),
                ));
            });
        });

    commands
        .spawn((
            AristeiaUiRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Px(110.0),
                width: Val::Px(settings.player.aristeia_bar_width),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.04, 0.82)),
        ))
        .with_children(|root| {
            root.spawn((
                GlobalAristeiaValueText,
                Text::new("GLOBAL ARISTEIA 0 / 0"),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(settings.player.aristeia_bar_height),
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.12, 0.12, 0.14)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    GlobalAristeiaBarFill,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.75, 0.2, 0.95)),
                ));
            });
        });
}

fn despawn_aristeia_ui(mut commands: Commands, roots: Query<Entity, With<AristeiaUiRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn update_aristeia_ui(
    local_player: Query<&PlayerAristeia, With<Predicted>>,
    global_aristeia: Query<&GlobalAristeia>,

    mut personal_text_query: Query<
        &mut Text,
        (With<AristeiaValueText>, Without<GlobalAristeiaValueText>),
    >,

    mut global_text_query: Query<
        &mut Text,
        (With<GlobalAristeiaValueText>, Without<AristeiaValueText>),
    >,

    mut personal_bar_query: Query<
        &mut Node,
        (With<AristeiaBarFill>, Without<GlobalAristeiaBarFill>),
    >,

    mut global_bar_query: Query<&mut Node, (With<GlobalAristeiaBarFill>, Without<AristeiaBarFill>)>,
) {
    let Ok(mut personal_text) = personal_text_query.single_mut() else {
        return;
    };

    let Ok(mut global_text) = global_text_query.single_mut() else {
        return;
    };

    let Ok(mut personal_bar) = personal_bar_query.single_mut() else {
        return;
    };

    let Ok(mut global_bar) = global_bar_query.single_mut() else {
        return;
    };

    if let Ok(aristeia) = local_player.single() {
        personal_text.0 = format!("ARISTEIA {}", aristeia.current);

        let fraction = if aristeia.maximum_ticks == 0 {
            0.0
        } else {
            (aristeia.remaining_ticks as f32 / aristeia.maximum_ticks as f32).clamp(0.0, 1.0)
        };

        personal_bar.width = Val::Percent(fraction * 100.0);
    } else {
        personal_text.0 = "ARISTEIA 0".to_string();
        personal_bar.width = Val::Percent(0.0);
    }

    if let Ok(global) = global_aristeia.single() {
        global_text.0 = format!("GLOBAL ARISTEIA {} / {}", global.current, global.maximum,);

        let fraction = if global.maximum == 0 {
            0.0
        } else {
            (global.current as f32 / global.maximum as f32).clamp(0.0, 1.0)
        };

        global_bar.width = Val::Percent(fraction * 100.0);
    } else {
        global_text.0 = "GLOBAL ARISTEIA 0 / 0".to_string();
        global_bar.width = Val::Percent(0.0);
    }
}
