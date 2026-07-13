use crate::app::AppState;
use bevy::prelude::*;

#[derive(Clone)]
pub struct HelpTextPlugin;

impl Plugin for HelpTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
        app.add_systems(OnEnter(AppState::Playing), setup_instructions);
        app.add_systems(OnExit(AppState::Playing), cleanup_instructions);
    }
}

fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[derive(Component)]
struct InstructionsText;

fn setup_instructions(mut commands: Commands) {
    commands.spawn((
        InstructionsText,
        Name::new("Input Control Instructions"),
        Text::new(
            "Move with WASD\nAim with Mouse\nHold Left Click to Fire\nN spawns an enemy (host/server debug)\nShift uses Skill",
        ),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn cleanup_instructions(mut commands: Commands, texts: Query<Entity, With<InstructionsText>>) {
    for entity in &texts {
        commands.entity(entity).despawn();
    }
}
