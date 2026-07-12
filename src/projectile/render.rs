use bevy::{color::Color, ecs::system::Query, gizmos::gizmos::Gizmos, math::Isometry2d};

use crate::{projectile::shared, protocol::ProjectilePosition};

pub(crate) fn draw_projectiles(mut gizmos: Gizmos, projectiles: Query<&ProjectilePosition>) {
    for position in &projectiles {
        gizmos.circle_2d(
            Isometry2d::from_translation(position.0),
            shared::PROJECTILE_RADIUS,
            Color::srgb(1.0, 0.85, 0.2),
        );
    }
}
