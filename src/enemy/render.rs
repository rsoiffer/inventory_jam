use crate::app::game_is_active;
use crate::enemy::protocol::{EnemyHealth, EnemyKind, EnemyPosition, EnemySpawnerPosition};
use crate::settings::GameSettings;
use bevy::prelude::*;
use bevy::sprite::Anchor;

const ENEMY_SPRITE_Z: f32 = 6.0;
const ENEMY_HEALTH_BAR_Z: f32 = 11.0;
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
                ensure_enemy_health_bars,
                sync_enemy_health_bars,
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
struct EnemyHealthBarBackground {
    enemy: Entity,
}

#[derive(Component)]
struct EnemyHealthBarFill {
    enemy: Entity,
}

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

fn enemy_health_bar_layout(
    settings: &GameSettings,
    kind: EnemyKind,
    position: Vec2,
) -> (Vec2, f32) {
    let width = kind.display_size(settings.enemy.size);
    let height = settings.player.health_bar_height;
    (Vec2::new(width, height), position.y + width + height)
}

fn ensure_enemy_health_bars(
    mut commands: Commands,
    settings: Res<GameSettings>,
    enemies: Query<(Entity, &EnemyPosition, &EnemyKind)>,
    backgrounds: Query<&EnemyHealthBarBackground>,
) {
    for (enemy, position, kind) in &enemies {
        if backgrounds.iter().any(|bar| bar.enemy == enemy) {
            continue;
        }

        let (bar_size, center_y) = enemy_health_bar_layout(&settings, *kind, position.0);
        let center = Vec2::new(position.0.x, center_y);

        commands.spawn((
            EnemyHealthBarBackground { enemy },
            Sprite::from_color(Color::WHITE, bar_size),
            Transform::from_translation(center.extend(ENEMY_HEALTH_BAR_Z)),
        ));

        let mut fill = Sprite::from_color(Color::WHITE, bar_size);
        fill.color = Color::srgb(0.9, 0.2, 0.2);
        commands.spawn((
            EnemyHealthBarFill { enemy },
            fill,
            Transform::from_translation(center.extend(ENEMY_HEALTH_BAR_Z + 0.1)),
        ));
    }
}

fn sync_enemy_health_bars(
    settings: Res<GameSettings>,
    mut commands: Commands,
    enemies: Query<(&EnemyPosition, &EnemyHealth, &EnemyKind)>,
    mut backgrounds: Query<
        (
            Entity,
            &EnemyHealthBarBackground,
            &mut Sprite,
            &mut Transform,
        ),
        Without<EnemyHealthBarFill>,
    >,
    mut fills: Query<
        (Entity, &EnemyHealthBarFill, &mut Sprite, &mut Transform),
        Without<EnemyHealthBarBackground>,
    >,
) {
    for (entity, bar, mut sprite, mut transform) in &mut backgrounds {
        let Ok((position, _, kind)) = enemies.get(bar.enemy) else {
            commands.entity(entity).despawn();
            continue;
        };

        let (full_size, center_y) = enemy_health_bar_layout(&settings, *kind, position.0);
        sprite.color = Color::srgb(0.15, 0.05, 0.05);
        sprite.custom_size = Some(full_size);
        transform.translation = Vec2::new(position.0.x, center_y).extend(ENEMY_HEALTH_BAR_Z);
    }

    for (entity, bar, mut sprite, mut transform) in &mut fills {
        let Ok((position, health, kind)) = enemies.get(bar.enemy) else {
            commands.entity(entity).despawn();
            continue;
        };

        let (full_size, center_y) = enemy_health_bar_layout(&settings, *kind, position.0);
        let health_fraction = if health.maximum == 0 {
            0.0
        } else {
            (health.current as f32 / health.maximum as f32).clamp(0.0, 1.0)
        };
        let fill_size = Vec2::new(full_size.x * health_fraction, full_size.y);
        sprite.color = Color::srgb(0.9, 0.2, 0.2);
        sprite.custom_size = Some(fill_size);
        transform.translation =
            Vec2::new(position.0.x + (fill_size.x - full_size.x) * 0.5, center_y)
                .extend(ENEMY_HEALTH_BAR_Z + 0.1);
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
