use crate::app::game_is_active;
use crate::shared::{ARENA_WORLD_BOUNDS, SHOP_WORLD_BOUNDS};
use bevy::app::{App, Plugin, Update};
use bevy::color::Color;
use bevy::math::{Isometry2d, Rect, Vec2};
use bevy::prelude::{Gizmos, IntoScheduleConfigs};

const GRID_SPACING: f32 = 100.0;

pub struct WorldRenderPlugin;

impl Plugin for WorldRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_test_worlds.run_if(game_is_active));
    }
}

fn draw_test_worlds(mut gizmos: Gizmos) {
    draw_test_world(&mut gizmos, ARENA_WORLD_BOUNDS);
    draw_test_world(&mut gizmos, SHOP_WORLD_BOUNDS);
}

/// Draws a simple top-down test arena with a grid and visible boundaries.
fn draw_test_world(gizmos: &mut Gizmos, bounds: Rect) {
    let grid_color = Color::srgba(0.35, 0.38, 0.42, 0.35);
    let axis_color = Color::srgba(0.7, 0.72, 0.75, 0.7);
    let boundary_color = Color::srgb(0.95, 0.25, 0.2);

    // Vertical grid lines
    let mut x = bounds.min.x;
    while x <= bounds.max.x {
        let color = if x.abs() < f32::EPSILON {
            axis_color
        } else {
            grid_color
        };
        gizmos.line_2d(
            Vec2::new(x, bounds.min.y),
            Vec2::new(x, bounds.max.y),
            color,
        );
        x += GRID_SPACING;
    }

    // Horizontal grid lines
    let mut y = bounds.min.y;
    while y <= bounds.max.y {
        let color = if y.abs() < f32::EPSILON {
            axis_color
        } else {
            grid_color
        };
        gizmos.line_2d(
            Vec2::new(bounds.min.x, y),
            Vec2::new(bounds.max.x, y),
            color,
        );
        y += GRID_SPACING;
    }

    // Boundary rectangle
    gizmos.rect_2d(
        Isometry2d::from_translation(bounds.center()),
        bounds.size(),
        boundary_color,
    );
}
