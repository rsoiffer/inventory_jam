use crate::app::ClientState;
use crate::debug_stats::ServerDebugStats;
use crate::networking::FIXED_TIMESTEP_HZ;
use crate::settings::GameSettings;
use bevy::prelude::*;

pub struct DebugPanelPlugin;

impl Plugin for DebugPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ClientState::Playing), spawn_debug_panel);
        app.add_systems(OnExit(ClientState::Playing), despawn_debug_panel);
        app.add_systems(
            Update,
            update_debug_panel.run_if(in_state(ClientState::Playing)),
        );
    }
}

#[derive(Component)]
struct DebugPanelRoot;

#[derive(Component)]
struct DebugPanelText;

fn spawn_debug_panel(
    mut commands: Commands,
    settings: Res<GameSettings>,
    existing: Query<Entity, With<DebugPanelRoot>>,
) {
    if !existing.is_empty() {
        return;
    }

    commands
        .spawn((
            DebugPanelRoot,
            Name::new("Server Debug Stats Panel"),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(settings.hud.inset),
                top: Val::Px(settings.hud.inset),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(settings.hud.row_gap),
                padding: UiRect::all(Val::Px(settings.hud.padding)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.04, 0.82)),
        ))
        .with_children(|root| {
            root.spawn((
                DebugPanelText,
                Text::new(format!(
                    "Sim Hz — / {}\nPlayers —\nEnemies —",
                    FIXED_TIMESTEP_HZ as u32
                )),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn despawn_debug_panel(mut commands: Commands, roots: Query<Entity, With<DebugPanelRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn update_debug_panel(
    stats: Query<&ServerDebugStats>,
    mut text: Query<&mut Text, With<DebugPanelText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };

    let target = FIXED_TIMESTEP_HZ as u32;
    text.0 = match stats.single() {
        Ok(stats) => format!(
            "Sim Hz {:.1}/{target}\nPlayers {}\nEnemies {}",
            stats.tick_hz, stats.players, stats.enemies
        ),
        Err(_) => format!("Sim Hz —/{target}\nPlayers —\nEnemies —"),
    };
}
