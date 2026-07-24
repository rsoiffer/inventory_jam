use crate::app::game_is_active;
use crate::portal::{PortalKind, PortalPosition};
use bevy::prelude::*;
use bevy::sprite::Anchor;

pub struct PortalRenderPlugin;

impl Plugin for PortalRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_portal_visual_assets);
        app.add_systems(
            Update,
            (
                ensure_portal_sprites,
                sync_portal_sprites,
                animate_portal_sprites,
            )
                .chain()
                .run_if(game_is_active),
        );
    }
}

#[derive(Resource)]
struct PortalVisualAssets {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    frame_count: usize,
}

#[derive(Component)]
struct PortalSpriteVisual {
    frame_count: usize,
}

#[derive(Component)]
struct PortalSpriteAnimation(Timer);

const PORTAL_SPRITE_Z: f32 = 3.0;
const PORTAL_FRAME_PIXELS: UVec2 = UVec2::new(48, 32);
const PORTAL_FRAME_SECONDS: f32 = 0.14;
const PORTAL_FRAME_COUNT: usize = 4;

fn load_portal_visual_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(PortalVisualAssets {
        image: asset_server.load("world_tiles/spr_gate_open.png"),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            PORTAL_FRAME_PIXELS,
            PORTAL_FRAME_COUNT as u32,
            1,
            None,
            None,
        )),
        frame_count: PORTAL_FRAME_COUNT,
    });
}

fn ensure_portal_sprites(
    mut commands: Commands,
    assets: Res<PortalVisualAssets>,
    portals: Query<
        Entity,
        (
            With<PortalKind>,
            With<PortalPosition>,
            Without<PortalSpriteVisual>,
        ),
    >,
) {
    for entity in &portals {
        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                assets.image.clone(),
                TextureAtlas {
                    layout: assets.layout.clone(),
                    index: 0,
                },
            ),
            Anchor::CENTER,
            Transform {
                translation: Vec3::new(0.0, 0.0, PORTAL_SPRITE_Z),
                ..default()
            },
            PortalSpriteVisual {
                frame_count: assets.frame_count,
            },
            PortalSpriteAnimation(Timer::from_seconds(
                PORTAL_FRAME_SECONDS,
                TimerMode::Repeating,
            )),
        ));
    }
}

fn sync_portal_sprites(
    mut portals: Query<(&PortalPosition, &mut Transform), With<PortalSpriteVisual>>,
) {
    for (position, mut transform) in &mut portals {
        transform.translation = position.0.extend(PORTAL_SPRITE_Z);
    }
}

fn animate_portal_sprites(
    time: Res<Time>,
    mut portals: Query<(&PortalSpriteVisual, &mut PortalSpriteAnimation, &mut Sprite)>,
) {
    for (visual, mut animation, mut sprite) in &mut portals {
        animation.0.tick(time.delta());
        if !animation.0.just_finished() {
            continue;
        }

        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };
        atlas.index = (atlas.index + 1) % visual.frame_count;
    }
}
