use bevy::prelude::*;

use crate::fragment::shared::{self as fragment_shared, FragmentPosition};

pub(crate) fn draw_fragments(mut gizmos: Gizmos, fragments: Query<&FragmentPosition>) {
    for position in &fragments {
        gizmos.circle_2d(
            Isometry2d::from_translation(position.0),
            fragment_shared::FRAGMENT_RADIUS,
            Color::srgb(0.22, 0.35, 0.45),
        );
    }
}
