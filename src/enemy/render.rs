use crate::app::game_is_active;
use crate::enemy::protocol::{EnemyHealth, EnemyKind, EnemyPosition, EnemySpawnerPosition};
use crate::settings::GameSettings;
use bevy::prelude::*;
use bevy::sprite::Anchor;

const ENEMY_SPRITE_Z: f32 = 6.0;
const SPAWNER_SPRITE_Z: f32 = 4.0;
const SPAWNER_FRAME_PIXELS: UVec2 = UVec2::new(48, 32);
const IDLE_FRAME_SECONDS: f32 = 0.12;
const FACING_MOVE_THRESHOLD: f32 = 0.01;

pub struct EnemyRenderPlugin;

impl Plugin for EnemyRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_enemy_visual_assets);
        app.add_systems(
            Update,
            (
                ensure_enemy_sprites,
                sync_enemy_sprites,
                animate_enemy_sprites,
                draw_health_bars,
                ensure_spawner_sprites,
                sync_spawner_sprites,
                animate_spawner_sprites,
            )
                .chain()
                .run_if(game_is_active),
        );
    }
}

#[derive(Clone)]
struct IdleSheet {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    frame_count: usize,
}

#[derive(Resource)]
struct EnemyVisualAssets {
    regular: IdleSheet,
    champion: IdleSheet,
    spawner: IdleSheet,
}

#[derive(Component)]
struct EnemySpriteVisual {
    previous_position: Option<Vec2>,
    frame_count: usize,
}

#[derive(Component)]
struct EnemySpriteAnimation(Timer);

#[derive(Component)]
struct SpawnerSpriteVisual {
    frame_count: usize,
}

#[derive(Component)]
struct SpawnerSpriteAnimation(Timer);

fn load_enemy_visual_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(EnemyVisualAssets {
        regular: IdleSheet {
            image: asset_server.load("enemy/spr_enemy_placeholder_idle.png"),
            layout: layouts.add(TextureAtlasLayout::from_grid(
                UVec2::splat(16),
                4,
                1,
                None,
                None,
            )),
            frame_count: 4,
        },
        champion: IdleSheet {
            image: asset_server.load("enemy/spr_champion_idle.png"),
            layout: layouts.add(TextureAtlasLayout::from_grid(
                UVec2::splat(64),
                4,
                1,
                None,
                None,
            )),
            frame_count: 4,
        },
        spawner: IdleSheet {
            image: asset_server.load("world_tiles/spr_altar_active.png"),
            layout: layouts.add(TextureAtlasLayout::from_grid(
                SPAWNER_FRAME_PIXELS,
                4,
                1,
                None,
                None,
            )),
            frame_count: 4,
        },
    });
}

fn ensure_enemy_sprites(
    mut commands: Commands,
    assets: Res<EnemyVisualAssets>,
    enemies: Query<(Entity, &EnemyKind), (With<EnemyPosition>, Without<EnemySpriteVisual>)>,
) {
    for (entity, kind) in &enemies {
        let sheet = match kind {
            EnemyKind::Regular => &assets.regular,
            EnemyKind::GrandChampion => &assets.champion,
        };

        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                sheet.image.clone(),
                TextureAtlas {
                    layout: sheet.layout.clone(),
                    index: 0,
                },
            ),
            Anchor::BOTTOM_CENTER,
            Transform {
                translation: Vec3::new(0.0, 0.0, ENEMY_SPRITE_Z),
                ..default()
            },
            EnemySpriteVisual {
                previous_position: None,
                frame_count: sheet.frame_count,
            },
            EnemySpriteAnimation(Timer::from_seconds(
                IDLE_FRAME_SECONDS,
                TimerMode::Repeating,
            )),
        ));
    }
}

fn sync_enemy_sprites(
    mut enemies: Query<(
        &EnemyPosition,
        &mut EnemySpriteVisual,
        &mut Sprite,
        &mut Transform,
    )>,
) {
    for (position, mut visual, mut sprite, mut transform) in &mut enemies {
        if let Some(previous) = visual.previous_position {
            let delta_x = position.0.x - previous.x;
            if delta_x.abs() > FACING_MOVE_THRESHOLD {
                sprite.flip_x = delta_x < 0.0;
            }
        }

        visual.previous_position = Some(position.0);
        transform.translation = position.0.extend(ENEMY_SPRITE_Z);
        transform.scale = Vec3::ONE;
    }
}

fn animate_enemy_sprites(
    time: Res<Time>,
    mut enemies: Query<(&EnemySpriteVisual, &mut EnemySpriteAnimation, &mut Sprite)>,
) {
    for (visual, mut animation, mut sprite) in &mut enemies {
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

fn ensure_spawner_sprites(
    mut commands: Commands,
    assets: Res<EnemyVisualAssets>,
    spawners: Query<Entity, (With<EnemySpawnerPosition>, Without<SpawnerSpriteVisual>)>,
) {
    let sheet = &assets.spawner;

    for entity in &spawners {
        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                sheet.image.clone(),
                TextureAtlas {
                    layout: sheet.layout.clone(),
                    index: 0,
                },
            ),
            Anchor::CENTER,
            Transform {
                translation: Vec3::new(0.0, 0.0, SPAWNER_SPRITE_Z),
                ..default()
            },
            SpawnerSpriteVisual {
                frame_count: sheet.frame_count,
            },
            SpawnerSpriteAnimation(Timer::from_seconds(
                IDLE_FRAME_SECONDS,
                TimerMode::Repeating,
            )),
        ));
    }
}

fn sync_spawner_sprites(
    mut spawners: Query<(&EnemySpawnerPosition, &mut Transform), With<SpawnerSpriteVisual>>,
) {
    for (position, mut transform) in &mut spawners {
        transform.translation = position.0.extend(SPAWNER_SPRITE_Z);
        transform.scale = Vec3::ONE;
    }
}

fn animate_spawner_sprites(
    time: Res<Time>,
    mut spawners: Query<(
        &SpawnerSpriteVisual,
        &mut SpawnerSpriteAnimation,
        &mut Sprite,
    )>,
) {
    for (visual, mut animation, mut sprite) in &mut spawners {
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

fn draw_health_bars(
    settings: Res<GameSettings>,
    mut gizmos: Gizmos,
    enemies: Query<(&EnemyPosition, &EnemyHealth, &EnemyKind)>,
) {
    for (position, health, kind) in &enemies {
        let health_fraction = if health.maximum == 0 {
            0.0
        } else {
            health.current as f32 / health.maximum as f32
        };

        let size = kind.display_size(settings.enemy.size);
        let bar_height = 1.6;
        let bar_size = Vec2::new(size * health_fraction, bar_height);
        let bar_center = position.0 + Vec2::new((bar_size.x - size) * 0.5, size + bar_height);
        gizmos.rect_2d(
            Isometry2d::from_translation(bar_center),
            bar_size,
            Color::srgb(0.9, 0.2, 0.2),
        );
    }
}
