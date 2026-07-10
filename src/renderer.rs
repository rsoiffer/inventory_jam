use crate::{protocol::*, shared};
use bevy::prelude::*;

/// Distance between debug grid lines in world units.
const GRID_SPACING: f32 = 100.0;
/// Half the number of grid lines along X (`WORLD_HALF_SIZE.x / GRID_SPACING`).
const GRID_X_HALF_STEPS: i16 = 8;
/// Half the number of grid lines along Y (`WORLD_HALF_SIZE.y / GRID_SPACING`).
const GRID_Y_HALF_STEPS: i16 = 6;

#[derive(Clone)]
/// Registers the 2D camera and gizmo renderer.
pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (init, setup_instructions));
        app.add_systems(Update, (draw_test_world, draw_boxes));
    }
}

/// Spawns the 2D camera used by the debug renderer.
fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Draws replicated player boxes from position and color components.
pub(crate) fn draw_boxes(mut gizmos: Gizmos, players: Query<(&PlayerPosition, &PlayerColor)>) {
    for (position, color) in &players {
        gizmos.rect_2d(
            Isometry2d::from_translation(position.0),
            Vec2::ONE * 50.0,
            color.0,
        );
    }
}

/// Draws a simple top-down test arena with a grid and visible boundaries.
fn draw_test_world(mut gizmos: Gizmos) {
    let half = shared::WORLD_HALF_SIZE;
    let grid_color = Color::srgba(0.35, 0.38, 0.42, 0.35);
    let axis_color = Color::srgba(0.7, 0.72, 0.75, 0.7);
    let boundary_color = Color::srgb(0.95, 0.25, 0.2);

    for i in -GRID_X_HALF_STEPS..=GRID_X_HALF_STEPS {
        let x = f32::from(i) * GRID_SPACING;
        let color = if i == 0 { axis_color } else { grid_color };
        gizmos.line_2d(Vec2::new(x, -half.y), Vec2::new(x, half.y), color);
    }

    for i in -GRID_Y_HALF_STEPS..=GRID_Y_HALF_STEPS {
        let y = f32::from(i) * GRID_SPACING;
        let color = if i == 0 { axis_color } else { grid_color };
        gizmos.line_2d(Vec2::new(-half.x, y), Vec2::new(half.x, y), color);
    }

    gizmos.rect_2d(Isometry2d::IDENTITY, half * 2.0, boundary_color);
}

/// Spawns on-screen control instructions for the debug client.
fn setup_instructions(mut commands: Commands) {
    commands.spawn((
        Text::new("Move with WASD\n Aim with Mouse \n Fire with Left Click \n Shift to use Skill"),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}
