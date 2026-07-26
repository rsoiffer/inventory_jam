use crate::app::ClientState;
use crate::enemy::{EnemyHealth, EnemyKind};
use crate::persistence::CachedPersistentState;
use crate::player::protocol::PlayerAristeia;
use crate::settings::GameSettings;
use crate::world::GlobalAristeia;
use bevy::prelude::*;
use lightyear::prediction::Predicted;
use lightyear::prelude::Controlled;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ClientState::Playing), spawn_hud);
        app.add_systems(OnExit(ClientState::Playing), despawn_hud);
        app.add_systems(Update, update_hud.run_if(in_state(ClientState::Playing)));
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

fn spawn_hud(
    mut commands: Commands,
    settings: Res<GameSettings>,
    existing: Query<Entity, With<HudRoot>>,
) {
    if !existing.is_empty() {
        return;
    }

    let bar_height = settings.hud.bar_height;

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
}

fn despawn_hud(mut commands: Commands, roots: Query<Entity, With<HudRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn update_hud(
    player_state: Query<&CachedPersistentState, With<Controlled>>,
    local_aristeia: Query<&PlayerAristeia, With<Predicted>>,
    global_aristeia: Query<&GlobalAristeia>,
    bosses: Query<(&EnemyHealth, &EnemyKind)>,
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
        if let Some(boss_health) = bosses.iter().find_map(|(health, kind)| {
            (*kind == EnemyKind::GrandChampion && health.current > 0).then_some(health)
        }) {
            let fraction = if boss_health.maximum == 0 {
                0.0
            } else {
                (boss_health.current as f32 / boss_health.maximum as f32).clamp(0.0, 1.0)
            };
            text.0 = format!(
                "Grand Champion {} / {}",
                boss_health.current, boss_health.maximum
            );
            bar.width = Val::Percent(fraction * 100.0);
            *color = BackgroundColor(Color::srgb(0.9, 0.2, 0.25));
        } else if let Ok(global) = global_aristeia.single() {
            text.0 = format!("Global {} / {}", global.current, global.maximum);
            bar.width = Val::Percent(global.fraction() * 100.0);
            *color = BackgroundColor(Color::srgb(0.75, 0.2, 0.95));
        } else {
            text.0 = "Global 0 / 0".to_string();
            bar.width = Val::Percent(0.0);
            *color = BackgroundColor(Color::srgb(0.75, 0.2, 0.95));
        }
    }
}
