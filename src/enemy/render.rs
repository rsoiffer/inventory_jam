use crate::app::game_is_active;
use crate::enemy::protocol::{ENEMY_SIZE, EnemyHealth, EnemyPosition};
use bevy::app::{App, Plugin, Update};
use bevy::prelude::IntoScheduleConfigs;
use bevy::{
    color::Color,
    ecs::system::Query,
    gizmos::gizmos::Gizmos,
    math::{Isometry2d, Vec2},
};

pub struct EnemyRenderPlugin;

impl Plugin for EnemyRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_enemy_boxes.run_if(game_is_active));
    }
}

fn draw_enemy_boxes(mut gizmos: Gizmos, enemies: Query<(&EnemyPosition, &EnemyHealth)>) {
    for (position, health) in &enemies {
        gizmos.rect_2d(
            Isometry2d::from_translation(position.0),
            Vec2::ONE * ENEMY_SIZE,
            Color::srgb(52. / 255., 235. / 255., 213. / 255.),
        );

        let health_fraction = if health.maximum == 0 {
            0.0
        } else {
            health.current as f32 / health.maximum as f32
        };

        let bar_size = Vec2::new(50.0 * health_fraction, 5.0);
        let bar_center = position.0 + Vec2::new((bar_size.x - 50.0) * 0.5, 34.0);
        gizmos.rect_2d(
            Isometry2d::from_translation(bar_center),
            bar_size,
            Color::srgb(0.9, 0.2, 0.2),
        );
    }
}
