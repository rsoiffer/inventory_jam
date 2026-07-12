use bevy::prelude::*;
use lightyear::prelude::Controlled;

use crate::fragment::{
    protocol::CarriedFragments,
    shared::{self as fragment_shared, FragmentPosition},
};

#[derive(Component)]
pub(crate) struct FragmentBalanceText;

pub(crate) fn draw_fragments(mut gizmos: Gizmos, fragments: Query<&FragmentPosition>) {
    for position in &fragments {
        gizmos.circle_2d(
            Isometry2d::from_translation(position.0),
            fragment_shared::FRAGMENT_RADIUS,
            Color::srgb(0.22, 0.35, 0.45),
        );
    }
}

pub(crate) fn setup_fragment_balance(mut commands: Commands) {
    commands.spawn((
        Name::new("Fragment Balance"),
        Text::new("Fragments: 0"),
        FragmentBalanceText,
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

pub(crate) fn update_fragment_balance(
    player: Query<&CarriedFragments, With<Controlled>>,
    mut text: Query<&mut Text, With<FragmentBalanceText>>,
) {
    let Ok(balance) = player.single() else {
        return;
    };

    let Ok(mut text) = text.single_mut() else {
        return;
    };

    text.0 = format!("Fragments: {}", balance.0);
}
