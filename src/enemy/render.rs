use crate::app::game_is_active;
use crate::enemy::protocol::{EnemyHealth, EnemyKind, EnemyPosition};
use crate::settings::GameSettings;
use bevy::prelude::*;

pub struct EnemyRenderPlugin;

impl Plugin for EnemyRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_enemy_visual_assets);
        app.add_systems(
            Update,
            (ensure_enemy_sprites, sync_enemy_sprites, draw_health_bars)
                .chain()
                .run_if(game_is_active),
        );
    }
}

#[derive(Resource)]
struct EnemyVisualAssets {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

fn load_enemy_visual_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(EnemyVisualAssets {
        image: asset_server.load("world_tiles/spr_tileset_arena_main.png"),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(16),
            8,
            11,
            None,
            None,
        )),
    });
}

fn ensure_enemy_sprites(
    mut commands: Commands,
    assets: Res<EnemyVisualAssets>,
    enemies: Query<(Entity, &EnemyKind), (With<EnemyPosition>, Without<Sprite>)>,
) {
    for (entity, kind) in &enemies {
        let (index, scale) = match kind {
            EnemyKind::Regular => (3, 2.25),
            EnemyKind::GrandChampion => (4, 5.0),
        };
        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                assets.image.clone(),
                TextureAtlas {
                    layout: assets.layout.clone(),
                    index,
                },
            ),
            Transform::from_scale(Vec3::splat(scale)),
        ));
    }
}

fn sync_enemy_sprites(mut enemies: Query<(&EnemyPosition, &mut Transform)>) {
    for (position, mut transform) in &mut enemies {
        transform.translation = position.0.extend(6.0);
    }
}

fn draw_health_bars(
    settings: Res<GameSettings>,
    mut gizmos: Gizmos,
    enemies: Query<(&EnemyPosition, &EnemyHealth)>,
) {
    for (position, health) in &enemies {
        let health_fraction = if health.maximum == 0 {
            0.0
        } else {
            health.current as f32 / health.maximum as f32
        };

        let bar_size = Vec2::new(settings.enemy.size * health_fraction, 5.0);
        let bar_center = position.0 + Vec2::new((bar_size.x - settings.enemy.size) * 0.5, 34.0);
        gizmos.rect_2d(
            Isometry2d::from_translation(bar_center),
            bar_size,
            Color::srgb(0.9, 0.2, 0.2),
        );
    }
}
