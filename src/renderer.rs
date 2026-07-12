use crate::enemy::render::draw_enemy_boxes;
use crate::{projectile::render::draw_projectiles, protocol::*, shared};
use bevy::prelude::*;

const GRID_SPACING: f32 = 100.0;

#[derive(Clone)]
pub struct ExampleRendererPlugin;

impl Plugin for ExampleRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (init, setup_instructions));

        app.add_systems(
            Update,
            (
                draw_test_world,
                draw_player_boxes,
                draw_projectiles,
                draw_enemy_boxes,
            ),
        );
    }
}
fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// System that draws the boxes of the player positions.
/// The components should be replicated from the server to the client
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

/// Draws a simple top-down test arena with a grid and visible boundaries.
fn draw_test_world(mut gizmos: Gizmos) {
    let half = shared::WORLD_HALF_SIZE;
    let grid_color = Color::srgba(0.35, 0.38, 0.42, 0.35);
    let axis_color = Color::srgba(0.7, 0.72, 0.75, 0.7);
    let boundary_color = Color::srgb(0.95, 0.25, 0.2);

    let mut x = -half.x;
    while x <= half.x {
        let color = if x.abs() < f32::EPSILON {
            axis_color
        } else {
            grid_color
        };
        gizmos.line_2d(Vec2::new(x, -half.y), Vec2::new(x, half.y), color);
        x += GRID_SPACING;
    }

    let mut y = -half.y;
    while y <= half.y {
        let color = if y.abs() < f32::EPSILON {
            axis_color
        } else {
            grid_color
        };
        gizmos.line_2d(Vec2::new(-half.x, y), Vec2::new(half.x, y), color);
        y += GRID_SPACING;
    }

    gizmos.rect_2d(Isometry2d::IDENTITY, half * 2.0, boundary_color);
}

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
