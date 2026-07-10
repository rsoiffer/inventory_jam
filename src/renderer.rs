use crate::protocol::*;
use bevy::prelude::*;

#[derive(Clone)]
/// Registers the 2D camera and gizmo renderer.
pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
        app.add_systems(Update, draw_boxes);
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
