use crate::app::{ClientState, game_is_active};
use crate::fragment::shared::FragmentPosition;
use crate::persistence::CachedPersistentState;
use bevy::prelude::*;
use lightyear::prelude::Controlled;

pub struct FragmentRenderPlugin;

impl Plugin for FragmentRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_fragment_visual_assets);
        app.add_systems(OnEnter(ClientState::Playing), setup_fragment_balance);
        app.add_systems(
            OnExit(ClientState::Playing),
            |mut commands: Commands, texts: Query<Entity, With<FragmentBalanceText>>| {
                for entity in &texts {
                    commands.entity(entity).despawn();
                }
            },
        );
        app.add_systems(
            Update,
            (
                ensure_fragment_sprites,
                sync_fragment_sprites,
                animate_fragment_sprites,
            )
                .chain()
                .run_if(game_is_active),
        );
        app.add_systems(
            Update,
            update_fragment_balance.run_if(in_state(ClientState::Playing)),
        );
    }
}

#[derive(Resource)]
struct FragmentVisualAssets {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

#[derive(Component)]
struct FragmentAnimation(Timer);

#[derive(Component)]
pub(crate) struct FragmentBalanceText;

fn load_fragment_visual_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(FragmentVisualAssets {
        image: asset_server.load("fragments/spr_vfx_part_pull.png"),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(16),
            8,
            1,
            None,
            None,
        )),
    });
}

fn ensure_fragment_sprites(
    mut commands: Commands,
    assets: Res<FragmentVisualAssets>,
    fragments: Query<Entity, (With<FragmentPosition>, Without<Sprite>)>,
) {
    for entity in &fragments {
        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                assets.image.clone(),
                TextureAtlas {
                    layout: assets.layout.clone(),
                    index: 0,
                },
            ),
            Transform::from_scale(Vec3::splat(1.25)),
            FragmentAnimation(Timer::from_seconds(0.08, TimerMode::Repeating)),
        ));
    }
}

fn sync_fragment_sprites(mut fragments: Query<(&FragmentPosition, &mut Transform)>) {
    for (position, mut transform) in &mut fragments {
        transform.translation = position.0.extend(7.0);
    }
}

fn animate_fragment_sprites(
    time: Res<Time>,
    mut fragments: Query<(&mut FragmentAnimation, &mut Sprite)>,
) {
    for (mut animation, mut sprite) in &mut fragments {
        animation.0.tick(time.delta());
        if !animation.0.just_finished() {
            continue;
        }
        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };
        atlas.index = (atlas.index + 1) % 8;
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
