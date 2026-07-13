use crate::app::{AppState, game_is_active};
use crate::fragment::{
    render,
    shared::{self as fragment_shared, FragmentPosition},
};
use crate::persistence::CachedPersistentState;
use bevy::prelude::*;
use lightyear::prelude::Controlled;

/// Installs fragment rendering without exposing rendering internals elsewhere.
pub struct FragmentRenderPlugin;

impl Plugin for FragmentRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Playing), render::setup_fragment_balance);
        app.add_systems(
            OnExit(AppState::Playing),
            |mut commands: Commands, texts: Query<Entity, With<render::FragmentBalanceText>>| {
                for entity in &texts {
                    commands.entity(entity).despawn();
                }
            },
        );

        app.add_systems(Update, render::draw_fragments.run_if(game_is_active));
        app.add_systems(
            Update,
            render::update_fragment_balance.run_if(in_state(AppState::Playing)),
        );
    }
}

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
    player: Query<&CachedPersistentState, With<Controlled>>,
    mut text: Query<&mut Text, With<FragmentBalanceText>>,
) {
    let Ok(state) = player.single() else {
        return;
    };

    let Ok(mut text) = text.single_mut() else {
        return;
    };

    text.0 = format!("Fragments: {}", state.fragment_count);
}
