use crate::app::game_is_active;
use crate::fragment::protocol::{Fragment, FragmentPhase};
use crate::settings::{GameSettings, WeaponSpriteSheetSettings};
use bevy::prelude::*;

pub struct FragmentRenderPlugin;

impl Plugin for FragmentRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_fragment_sprite_assets);
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
    }
}

#[derive(Clone)]
struct SpriteSheet {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    frame_count: usize,
    frame_seconds: f32,
}

#[derive(Resource)]
struct FragmentSpriteAssets {
    on_ground: SpriteSheet,
    starting_pull: SpriteSheet,
    moving: SpriteSheet,
    collecting_impact: SpriteSheet,
}

impl FragmentSpriteAssets {
    fn get(&self, phase: FragmentPhase) -> Option<&SpriteSheet> {
        match phase {
            FragmentPhase::OnGround => Some(&self.on_ground),
            FragmentPhase::StartingPull { .. } => Some(&self.starting_pull),
            FragmentPhase::Moving { .. } => Some(&self.moving),
            FragmentPhase::CollectingImpact { .. } => Some(&self.collecting_impact),
            FragmentPhase::Gone => None,
        }
    }
}

#[derive(Component)]
struct FragmentAnimation {
    timer: Timer,
    frame_count: usize,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct FragmentSpritePhase(FragmentPhase);

fn load_sheet(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sheet: &WeaponSpriteSheetSettings,
) -> SpriteSheet {
    SpriteSheet {
        image: asset_server.load(sheet.path.clone()),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            sheet.cell,
            sheet.frames,
            1,
            None,
            None,
        )),
        frame_count: sheet.frames as usize,
        frame_seconds: sheet.frame_seconds,
    }
}

fn load_fragment_sprite_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    settings: Res<GameSettings>,
) {
    let f = &settings.fragment;
    commands.insert_resource(FragmentSpriteAssets {
        on_ground: load_sheet(&asset_server, &mut layouts, &f.on_ground),
        starting_pull: load_sheet(&asset_server, &mut layouts, &f.starting_pull),
        moving: load_sheet(&asset_server, &mut layouts, &f.moving),
        collecting_impact: load_sheet(&asset_server, &mut layouts, &f.collecting_impact),
    });
}

fn ensure_fragment_sprites(
    mut commands: Commands,
    assets: Res<FragmentSpriteAssets>,
    settings: Res<GameSettings>,
    fragments: Query<(Entity, &Fragment, Option<&FragmentSpritePhase>)>,
) {
    for (entity, fragment, sprite_phase) in &fragments {
        let Some(sheet) = assets.get(fragment.phase) else {
            commands.entity(entity).insert(Visibility::Hidden);
            if sprite_phase.is_some() {
                commands
                    .entity(entity)
                    .remove::<(Sprite, FragmentAnimation, FragmentSpritePhase)>();
            }
            continue;
        };

        if sprite_phase.is_some_and(|phase| phase.0 == fragment.phase) {
            continue;
        }

        let mut sprite = Sprite::from_atlas_image(
            sheet.image.clone(),
            TextureAtlas {
                layout: sheet.layout.clone(),
                index: 0,
            },
        );
        sprite.color = settings.post_processing.fragment_tint;
        commands.entity(entity).insert((
            sprite,
            Visibility::Visible,
            FragmentSpritePhase(fragment.phase),
            FragmentAnimation {
                timer: Timer::from_seconds(sheet.frame_seconds, TimerMode::Repeating),
                frame_count: sheet.frame_count,
            },
        ));
    }
}

fn sync_fragment_sprites(
    mut fragments: Query<(&Fragment, &mut Transform, Option<&mut Visibility>)>,
) {
    for (fragment, mut transform, visibility) in &mut fragments {
        let hidden = matches!(fragment.phase, FragmentPhase::Gone);
        if let Some(mut visibility) = visibility {
            *visibility = if hidden {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
        }
        if hidden {
            continue;
        }

        transform.translation = fragment.position.extend(5.0);
        transform.rotation = if matches!(fragment.phase, FragmentPhase::Moving { .. })
            && fragment.movement.length_squared() > f32::EPSILON
        {
            Quat::from_rotation_z(fragment.movement.y.atan2(fragment.movement.x))
        } else {
            Quat::IDENTITY
        };
    }
}

fn animate_fragment_sprites(
    time: Res<Time>,
    mut fragments: Query<(&mut FragmentAnimation, &mut Sprite), With<FragmentSpritePhase>>,
) {
    for (mut animation, mut sprite) in &mut fragments {
        animation.timer.tick(time.delta());
        if animation.timer.just_finished()
            && let Some(atlas) = sprite.texture_atlas.as_mut()
        {
            atlas.index = (atlas.index + 1) % animation.frame_count;
        }
    }
}
