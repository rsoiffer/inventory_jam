use bevy::{color::Color, ecs::system::Query, gizmos::gizmos::Gizmos, math::Isometry2d};

use crate::fragment::shared::{self, FragmentPosition};

pub(crate) fn draw_fragments(mut gizmos: Gizmos, fragments: Query<&FragmentPosition>) {
    for position in &fragments {
        gizmos.circle_2d(
            Isometry2d::from_translation(position.0),
            shared::FRAGMENT_RADIUS,
            Color::srgb(0.22, 0.35, 0.45),
        );
    }
}
