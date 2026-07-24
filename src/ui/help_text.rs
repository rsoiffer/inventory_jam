use crate::app::{ClientState, ServerState};
use bevy::prelude::*;

#[derive(Clone)]
pub struct HelpTextPlugin;

impl Plugin for HelpTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
        app.add_systems(OnEnter(ClientState::Playing), setup_instructions);
        app.add_systems(OnEnter(ServerState::Hosting), server_instructions);
        app.add_systems(OnExit(ClientState::Playing), cleanup_instructions);
    }
}

fn init(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.16,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

#[derive(Component)]
struct InstructionsText;

fn setup_instructions(mut commands: Commands) {
    commands.spawn((
        InstructionsText,
        Name::new("Input Control Instructions"),
        Text::new(
            "Move with WASD\nAim with Mouse\nHold Left Click to Fire\n1-6 change weapons\nR/T teleport between the Arena and Pit\nN spawns an enemy (host/server debug)\nShift spawns a debug fragment pool",
        ),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}
fn server_instructions(mut commands: Commands) {
    commands.spawn((
        InstructionsText,
        Name::new("Server Camera Control Instructions"),
        Text::new(
            "Dedicated-server overview camera controls:\n- F: frame both Arena and Safezone\n- A/D or Left/Right: pan horizontally\n- W/S or Up/Down: pan vertically\n- Minus/Equals: zoom out/in\n- 0: reset to a 1:1 camera scale",
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
