use crate::app::ClientState;
use crate::enemy::{EnemyHealth, EnemyIdentity, EnemyKind, EnemyPosition};
use crate::persistence::CachedPersistentState;
use crate::player::protocol::PlayerAristeia;
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::settings::GameSettings;
use crate::world::GlobalAristeia;
use bevy::camera::{Camera, Camera2d};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use lightyear::prediction::Predicted;
use lightyear::prelude::Controlled;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ClientState::Playing), spawn_hud);
        app.add_systems(OnExit(ClientState::Playing), despawn_hud);
        app.add_systems(
            Update,
            (update_hud, update_boss_direction_indicator)
                .chain()
                .run_if(in_state(ClientState::Playing)),
        );
    }
}

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct FragmentHudText;

#[derive(Component)]
struct PersonalAristeiaText;

#[derive(Component)]
struct PersonalAristeiaBarFill;

#[derive(Component)]
struct GlobalAristeiaText;

#[derive(Component)]
struct GlobalAristeiaBarFill;

#[derive(Component)]
struct BossDirectionIndicator;

fn spawn_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    existing: Query<Entity, With<HudRoot>>,
) {
    if !existing.is_empty() {
        return;
    }

    let bar_height = settings.hud.bar_height;
    let indicator_size = settings.hud.boss_indicator_size;

    commands
        .spawn((
            HudRoot,
            Name::new("Gameplay HUD"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(settings.hud.inset),
                top: Val::Px(settings.hud.inset),
                width: Val::Px(settings.hud.width),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(settings.hud.row_gap),
                padding: UiRect::all(Val::Px(settings.hud.padding)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.04, 0.82)),
        ))
        .with_children(|root| {
            root.spawn((
                FragmentHudText,
                Text::new("Fragments 0"),
                TextFont {
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            root.spawn((Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },))
                .with_children(|row| {
                    row.spawn((
                        PersonalAristeiaText,
                        Text::new("Aristeia 0"),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(bar_height),
                            padding: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.12, 0.14)),
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            PersonalAristeiaBarFill,
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.95, 0.64, 0.12)),
                        ));
                    });
                });

            root.spawn((Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },))
                .with_children(|row| {
                    row.spawn((
                        GlobalAristeiaText,
                        Text::new("Global 0 / 0"),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(bar_height),
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
        });

    commands.spawn((
        BossDirectionIndicator,
        Name::new("Boss Direction Indicator"),
        ImageNode::new(asset_server.load(settings.hud.boss_indicator_path.clone()))
            .with_color(settings.hud.boss_indicator_color),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(indicator_size),
            height: Val::Px(indicator_size),
            ..default()
        },
        UiTransform::default(),
        Visibility::Hidden,
        ZIndex(20),
    ));
}

fn despawn_hud(
    mut commands: Commands,
    roots: Query<Entity, With<HudRoot>>,
    indicators: Query<Entity, With<BossDirectionIndicator>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    for indicator in &indicators {
        commands.entity(indicator).despawn();
    }
}

fn update_hud(
    player_state: Query<&CachedPersistentState, With<Controlled>>,
    local_aristeia: Query<&PlayerAristeia, With<Predicted>>,
    global_aristeia: Query<&GlobalAristeia>,
    bosses: Query<(&EnemyHealth, &EnemyIdentity)>,
    mut fragment_text: Query<
        &mut Text,
        (
            With<FragmentHudText>,
            Without<PersonalAristeiaText>,
            Without<GlobalAristeiaText>,
        ),
    >,
    mut personal_text: Query<
        &mut Text,
        (
            With<PersonalAristeiaText>,
            Without<FragmentHudText>,
            Without<GlobalAristeiaText>,
        ),
    >,
    mut global_text: Query<
        &mut Text,
        (
            With<GlobalAristeiaText>,
            Without<FragmentHudText>,
            Without<PersonalAristeiaText>,
        ),
    >,
    mut personal_bar: Query<
        &mut Node,
        (
            With<PersonalAristeiaBarFill>,
            Without<GlobalAristeiaBarFill>,
        ),
    >,
    mut global_bar: Query<
        (&mut Node, &mut BackgroundColor),
        (
            With<GlobalAristeiaBarFill>,
            Without<PersonalAristeiaBarFill>,
        ),
    >,
) {
    if let Ok(mut text) = fragment_text.single_mut() {
        let fragments = player_state
            .single()
            .map(|state| state.weapon(state.equipped_weapon_id).fragments)
            .unwrap_or(0);
        text.0 = format!("Fragments {fragments}");
    }

    if let (Ok(mut text), Ok(mut bar)) = (personal_text.single_mut(), personal_bar.single_mut()) {
        if let Ok(aristeia) = local_aristeia.single() {
            text.0 = format!("Aristeia {}", aristeia.current);
            bar.width = Val::Percent(aristeia.remaining_fraction() * 100.0);
        } else {
            text.0 = "Aristeia 0".to_string();
            bar.width = Val::Percent(0.0);
        }
    }

    if let (Ok(mut text), Ok((mut bar, mut color))) =
        (global_text.single_mut(), global_bar.single_mut())
    {
        if let Some(boss_health) = bosses.iter().find_map(|(health, identity)| {
            (identity.kind == EnemyKind::GrandChampion && health.current > 0).then_some(health)
        }) {
            let fraction = if boss_health.maximum == 0 {
                0.0
            } else {
                (boss_health.current as f32 / boss_health.maximum as f32).clamp(0.0, 1.0)
            };
            text.0 = format!("Champion {}/{}", boss_health.current, boss_health.maximum);
            bar.width = Val::Percent(fraction * 100.0);
            *color = BackgroundColor(Color::srgb(0.9, 0.2, 0.25));
        } else if let Ok(global) = global_aristeia.single() {
            text.0 = format!("Global {}/{}", global.current, global.maximum);
            bar.width = Val::Percent(global.fraction() * 100.0);
            *color = BackgroundColor(Color::srgb(0.75, 0.2, 0.95));
        } else {
            text.0 = "Global 0 / 0".to_string();
            bar.width = Val::Percent(0.0);
            *color = BackgroundColor(Color::srgb(0.75, 0.2, 0.95));
        }
    }
}

fn update_boss_direction_indicator(
    settings: Res<GameSettings>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    local_player: Query<&GameRoom, (With<Predicted>, With<Controlled>)>,
    bosses: Query<(&EnemyPosition, &EnemyHealth, &EnemyIdentity)>,
    mut indicator: Query<
        (&mut Node, &mut UiTransform, &mut Visibility),
        With<BossDirectionIndicator>,
    >,
) {
    let Ok((mut node, mut transform, mut visibility)) = indicator.single_mut() else {
        return;
    };
    let Ok(window) = window.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Ok((camera, camera_transform)) = camera.single() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let in_arena = local_player
        .iter()
        .any(|room| room.room == GameRooms::Arena);

    let Some(boss_position) = bosses.iter().find_map(|(position, health, identity)| {
        (in_arena && identity.kind == EnemyKind::GrandChampion && health.current > 0)
            .then_some(position.0)
    }) else {
        *visibility = Visibility::Hidden;
        return;
    };

    let Ok(screen_pos) = camera.world_to_viewport(camera_transform, boss_position.extend(0.0))
    else {
        *visibility = Visibility::Hidden;
        return;
    };

    let screen_size = Vec2::new(window.width(), window.height());
    let margin = settings.hud.boss_indicator_margin;
    let size = settings.hud.boss_indicator_size;
    let (edge_pos, onscreen) = clamp_to_screen_edge(screen_pos, screen_size, margin);
    if onscreen {
        *visibility = Visibility::Hidden;
        return;
    }

    let center = screen_size * 0.5;
    let direction = (screen_pos - center).normalize_or_zero();
    let angle = if direction == Vec2::ZERO {
        0.0
    } else {
        direction.y.atan2(direction.x)
    };

    node.left = Val::Px(edge_pos.x - size * 0.5);
    node.top = Val::Px(edge_pos.y - size * 0.5);
    transform.rotation = Rot2::radians(angle);
    *visibility = Visibility::Visible;
}

fn clamp_to_screen_edge(screen_pos: Vec2, screen_size: Vec2, margin: f32) -> (Vec2, bool) {
    let inset_min = Vec2::splat(margin);
    let inset_max = (screen_size - inset_min).max(inset_min);
    let onscreen = screen_pos.cmpge(inset_min).all() && screen_pos.cmple(inset_max).all();
    if onscreen {
        return (screen_pos, true);
    }

    let center = screen_size * 0.5;
    let direction = (screen_pos - center).normalize_or_zero();
    if direction == Vec2::ZERO {
        return (center, true);
    }

    let max_offset = (inset_max - center).max(Vec2::splat(1.0));
    let tx = if direction.x.abs() < f32::EPSILON {
        f32::INFINITY
    } else {
        max_offset.x / direction.x.abs()
    };
    let ty = if direction.y.abs() < f32::EPSILON {
        f32::INFINITY
    } else {
        max_offset.y / direction.y.abs()
    };
    (center + direction * tx.min(ty), false)
}
