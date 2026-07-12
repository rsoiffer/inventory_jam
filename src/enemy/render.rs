use bevy::{
    color::Color,
    ecs::system::Query,
    gizmos::gizmos::Gizmos,
    math::{Isometry2d, Vec2},
};

use crate::enemy::protocol::{ENEMY_SIZE, EnemyPosition};

pub(crate) fn draw_enemy_boxes(mut gizmos: Gizmos, enemies: Query<&EnemyPosition>) {
    for enemypos in &enemies {
        gizmos.rect_2d(
            Isometry2d::from_translation(enemypos.0),
            Vec2::ONE * ENEMY_SIZE,
            Color::srgb(52. / 255., 235. / 255., 213. / 255.),
        );
    }
}
