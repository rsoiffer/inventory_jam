use crate::GameState;
use crate::debug_log::agent_log;
use crate::loading::TextureAssets;
use crate::protocol::{Inputs, PlayerColor, PlayerPosition};
use bevy::ecs::query::Or;
use bevy::prelude::*;
use lightyear::frame_interpolation::{FrameInterpolate, FrameInterpolationSystems};
use lightyear::prelude::input::native::InputMarker;
use lightyear::prelude::{Interpolated, Predicted};

pub struct PlayerPlugin;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct GameCamera;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct PlayerVisual;

pub(crate) fn clear_player_visual(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .remove::<(PlayerVisual, FrameInterpolate<PlayerPosition>, Sprite)>();
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<GameCamera>()
            .register_type::<PlayerVisual>()
            .add_systems(OnEnter(GameState::Playing), spawn_camera)
            .add_systems(
                PostUpdate,
                sync_player_transforms
                    .after(FrameInterpolationSystems::Interpolate)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                spawn_player_visuals.run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_camera);
    }
}

fn spawn_camera(mut commands: Commands, cameras: Query<Entity, With<Camera2d>>) {
    if let Ok(camera) = cameras.single() {
        commands.entity(camera).insert(GameCamera);
        return;
    }

    commands.spawn((Camera2d, GameCamera));
}

fn cleanup_camera(mut commands: Commands, cameras: Query<Entity, With<GameCamera>>) {
    for camera in &cameras {
        commands.entity(camera).remove::<GameCamera>();
    }
}

fn spawn_player_visuals(
    mut commands: Commands,
    textures: Res<TextureAssets>,
    local_players: Query<
        (Entity, &PlayerPosition, &PlayerColor),
        (
            Without<PlayerVisual>,
            Or<(
                With<InputMarker<Inputs>>,
                (With<Interpolated>, Without<Predicted>),
            )>,
        ),
    >,
    markers: Query<(), With<InputMarker<Inputs>>>,
    interpolated: Query<(), With<Interpolated>>,
    has_input_marker: Query<(), With<InputMarker<Inputs>>>,
) {
    // #region agent log
    if !local_players.is_empty() {
        agent_log(
            "F",
            "player.rs:spawn_player_visuals",
            "spawning player visuals",
            &format!(
                r#"{{"spawn_count":{},"input_marker_count":{},"interpolated_count":{}}}"#,
                local_players.iter().count(),
                markers.iter().count(),
                interpolated.iter().count(),
            ),
        );
    }
    // #endregion

    for (entity, position, color) in &local_players {
        let sprite = Sprite {
            image: textures.bevy.clone(),
            color: color.0,
            ..default()
        };
        let transform = Transform::from_translation(position.0.extend(1.0));
        if has_input_marker.get(entity).is_ok() {
            commands.entity(entity).insert((
                PlayerVisual,
                FrameInterpolate::<PlayerPosition>::default(),
                sprite,
                transform,
            ));
        } else {
            commands.entity(entity).insert((PlayerVisual, sprite, transform));
        }
    }
}

fn sync_player_transforms(
    mut players: Query<(&PlayerPosition, &mut Transform), With<PlayerVisual>>,
) {
    for (position, mut transform) in &mut players {
        transform.translation.x = position.0.x;
        transform.translation.y = position.0.y;
    }
}
