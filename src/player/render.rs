use crate::app::{ClientState, game_is_active};
use crate::persistence::CachedPersistentState;
use crate::player::PlayerUsername;
use crate::player::protocol::{
    LocalAimInput, PlayerAimDirection, PlayerAristeia, PlayerHealth, PlayerPosition, PlayerVisual,
    SmoothedAimDirection,
};
use crate::settings::GameSettings;
use crate::world::GlobalAristeia;

use bevy::app::{App, Plugin, Startup, Update};
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::camera::{Camera, Camera2d};
use bevy::color::Color;
use bevy::ecs::query::Without;
use bevy::image::Image;
use bevy::math::{StableInterpolate, UVec2, Vec2, Vec3};
use bevy::prelude::{
    AlignItems, BackgroundColor, Commands, Component, Entity, FlexDirection, FontSize,
    GlobalTransform, IntoScheduleConfigs, JustifyContent, Node, OnEnter, OnExit, PositionType,
    Query, Res, ResMut, Resource, Single, Sprite, Text, Text2d, TextColor, TextFont, TextureAtlas,
    TextureAtlasLayout, Time, Timer, TimerMode, Transform, UiRect, Val, Window, With, default,
    in_state,
};
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

use lightyear::prediction::Predicted;

const PLAYER_SPRITE_Z: f32 = 5.0;
const PLAYER_IDLE_FRAME_COUNT: usize = 2;
const PLAYER_IDLE_FRAME_SECONDS: f32 = 0.12;
const FACING_MOVE_THRESHOLD: f32 = 0.01;
const PLAYER_IDLE_FRAME_SIZE: UVec2 = UVec2::new(24, 24);

pub struct PlayerRenderPlugin;

impl Plugin for PlayerRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_player_visual_assets);
        app.add_systems(OnEnter(ClientState::Playing), spawn_aristeia_ui);

        app.add_systems(
            OnExit(ClientState::Playing),
            (despawn_aristeia_ui, despawn_death_overlay),
        );

        app.add_systems(
            Update,
            (
                ensure_player_sprites,
                sync_player_sprites,
                animate_player_sprites,
                ensure_player_health_bars,
                sync_player_health_bars,
                sync_username_labels,
            )
                .chain()
                .run_if(game_is_active),
        );

        app.add_systems(
            Update,
            (update_aristeia_ui, sync_death_overlay).run_if(in_state(ClientState::Playing)),
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

#[derive(Resource)]
struct PlayerVisualAssets {
    idle_image: Handle<Image>,
    idle_layout: Handle<TextureAtlasLayout>,
}

#[derive(Component)]
struct PlayerBodySprite {
    player: Entity,
    previous_position: Option<Vec2>,
}

#[derive(Component)]
struct PlayerSpriteAnimation {
    timer: Timer,
    frame_count: usize,
}

#[derive(Component)]
struct PlayerHealthBarBackground {
    player: Entity,
}

#[derive(Component)]
struct PlayerHealthBarFill {
    player: Entity,
}

const PLAYER_HEALTH_BAR_Z: f32 = 11.0;

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

#[derive(Component)]
struct DeathOverlayRoot;

fn load_player_visual_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(PlayerVisualAssets {
        idle_image: asset_server.load("player/spr_gladiator_idle.png"),
        idle_layout: layouts.add(TextureAtlasLayout::from_grid(
            PLAYER_IDLE_FRAME_SIZE,
            PLAYER_IDLE_FRAME_COUNT as u32,
            1,
            None,
            None,
        )),
    });
}

fn ensure_player_sprites(
    mut commands: Commands,
    assets: Res<PlayerVisualAssets>,
    players: Query<Entity, With<PlayerVisual>>,
    bodies: Query<&PlayerBodySprite>,
) {
    for player in &players {
        if bodies.iter().any(|body| body.player == player) {
            continue;
        }

        commands.spawn((
            PlayerBodySprite {
                player,
                previous_position: None,
            },
            Sprite::from_atlas_image(
                assets.idle_image.clone(),
                TextureAtlas {
                    layout: assets.idle_layout.clone(),
                    index: 0,
                },
            ),
            Anchor::CENTER,
            Transform {
                translation: Vec3::new(0.0, 0.0, PLAYER_SPRITE_Z),
                ..default()
            },
            PlayerSpriteAnimation {
                timer: Timer::from_seconds(PLAYER_IDLE_FRAME_SECONDS, TimerMode::Repeating),
                frame_count: PLAYER_IDLE_FRAME_COUNT,
            },
        ));
    }
}

fn sync_player_sprites(
    players: Query<&PlayerPosition, With<PlayerVisual>>,
    mut bodies: Query<(Entity, &mut PlayerBodySprite, &mut Sprite, &mut Transform)>,
    mut commands: Commands,
) {
    for (body_entity, mut body, mut sprite, mut transform) in &mut bodies {
        let Ok(position) = players.get(body.player) else {
            commands.entity(body_entity).despawn();
            continue;
        };

        if let Some(previous) = body.previous_position {
            let delta_x = position.0.x - previous.x;
            if delta_x.abs() > FACING_MOVE_THRESHOLD {
                sprite.flip_x = delta_x < 0.0;
            }
        }

        body.previous_position = Some(position.0);
        transform.translation = position.0.extend(PLAYER_SPRITE_Z);
        transform.scale = Vec3::ONE;
    }
}

fn animate_player_sprites(
    time: Res<Time>,
    mut bodies: Query<(&mut PlayerSpriteAnimation, &mut Sprite)>,
) {
    for (mut animation, mut sprite) in &mut bodies {
        animation.timer.tick(time.delta());
        if !animation.timer.just_finished() {
            continue;
        }

        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };
        atlas.index = (atlas.index + 1) % animation.frame_count;
    }
}

fn ensure_player_health_bars(
    mut commands: Commands,
    settings: Res<GameSettings>,
    players: Query<(Entity, &PlayerPosition), With<PlayerVisual>>,
    backgrounds: Query<&PlayerHealthBarBackground>,
) {
    let bar_size = Vec2::new(
        settings.player.health_bar_width,
        settings.player.health_bar_height,
    );

    for (player, position) in &players {
        if backgrounds.iter().any(|bar| bar.player == player) {
            continue;
        }

        let center = position.0 + Vec2::new(0.0, settings.player.health_bar_offset_y);

        commands.spawn((
            PlayerHealthBarBackground { player },
            Sprite::from_color(Color::WHITE, bar_size),
            Transform::from_translation(center.extend(PLAYER_HEALTH_BAR_Z)).with_scale(Vec3::ONE),
        ));

        let mut fill = Sprite::from_color(Color::WHITE, bar_size);
        fill.color = Color::srgb(0.9, 0.2, 0.2);
        commands.spawn((
            PlayerHealthBarFill { player },
            fill,
            Transform::from_translation(center.extend(PLAYER_HEALTH_BAR_Z + 0.1)),
        ));
    }
}

fn sync_player_health_bars(
    settings: Res<GameSettings>,
    mut commands: Commands,
    players: Query<(&PlayerPosition, &PlayerHealth), With<PlayerVisual>>,
    mut backgrounds: Query<
        (
            Entity,
            &PlayerHealthBarBackground,
            &mut Sprite,
            &mut Transform,
        ),
        Without<PlayerHealthBarFill>,
    >,
    mut fills: Query<
        (Entity, &PlayerHealthBarFill, &mut Sprite, &mut Transform),
        Without<PlayerHealthBarBackground>,
    >,
) {
    let full_size = Vec2::new(
        settings.player.health_bar_width,
        settings.player.health_bar_height,
    );

    for (entity, bar, mut sprite, mut transform) in &mut backgrounds {
        let Ok((position, _)) = players.get(bar.player) else {
            commands.entity(entity).despawn();
            continue;
        };

        sprite.color = Color::srgb(0.15, 0.05, 0.05);
        sprite.custom_size = Some(full_size);
        transform.translation = (position.0 + Vec2::new(0.0, settings.player.health_bar_offset_y))
            .extend(PLAYER_HEALTH_BAR_Z);
    }

    for (entity, bar, mut sprite, mut transform) in &mut fills {
        let Ok((position, health)) = players.get(bar.player) else {
            commands.entity(entity).despawn();
            continue;
        };

        let health_fraction = if health.maximum == 0 {
            0.0
        } else {
            (health.current as f32 / health.maximum as f32).clamp(0.0, 1.0)
        };

        let fill_size = Vec2::new(full_size.x * health_fraction, full_size.y);
        sprite.color = Color::srgb(0.9, 0.2, 0.2);
        sprite.custom_size = Some(fill_size);
        transform.translation = (position.0
            + Vec2::new(
                (fill_size.x - full_size.x) * 0.5,
                settings.player.health_bar_offset_y,
            ))
        .extend(PLAYER_HEALTH_BAR_Z + 0.1);
    }
}

fn sync_username_labels(
    settings: Res<GameSettings>,
    mut commands: Commands,
    players: Query<
        (
            Entity,
            &PlayerPosition,
            &PlayerUsername,
            Option<&CachedPersistentState>,
        ),
        With<PlayerVisual>,
    >,
    mut labels: Query<(Entity, &UsernameLabel, &mut Transform, &mut Text2d)>,
) {
    for (player, position, username, cache) in &players {
        let label_text = match cache {
            Some(state) => format!(
                "{} Lv{}",
                username.0,
                state.weapon(state.equipped_weapon_id).level
            ),
            None => username.0.clone(),
        };
        let existing_label = labels
            .iter_mut()
            .find(|(_, label, _, _)| label.player == player);

        if let Some((_, _, mut transform, mut text)) = existing_label {
            transform.translation = position.0.extend(0.0) + settings.player.username_label_offset;

            if text.0 != label_text {
                text.0 = label_text;
            }

            continue;
        }

        commands.spawn((
            UsernameLabel { player },
            Text2d::new(label_text),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Anchor::BOTTOM_CENTER,
            Transform::from_translation(
                position.0.extend(0.0) + settings.player.username_label_offset,
            )
            .with_scale(Vec3::splat(0.48)),
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

    if camera.translation.truncate().distance_squared(player.0) > 96.0_f32.powi(2) {
        camera.translation = target;
    } else {
        camera.translation.smooth_nudge(
            &target,
            settings.player.camera_decay_rate,
            time.delta_secs(),
        );
    }
}

fn sample_cursor_aim(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    predicted_player: Single<&PlayerPosition, With<Predicted>>,
    touch_controls: Option<Res<crate::ui::TouchControlsEnabled>>,
    mut local_aim: ResMut<LocalAimInput>,
) {
    if touch_controls.is_some_and(|enabled| enabled.0) {
        return;
    }

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

fn sync_death_overlay(
    mut commands: Commands,
    local_player: Query<&PlayerHealth, With<Predicted>>,
    existing: Query<Entity, With<DeathOverlayRoot>>,
) {
    let is_dead = local_player.iter().any(|health| health.current == 0);

    if is_dead {
        if !existing.is_empty() {
            return;
        }

        commands
            .spawn((
                DeathOverlayRoot,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.02, 0.02, 0.72)),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("You died"),
                    TextFont {
                        font_size: FontSize::Px(56.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.3, 0.28)),
                ));
            });
    } else {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
    }
}

fn despawn_death_overlay(mut commands: Commands, roots: Query<Entity, With<DeathOverlayRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}
