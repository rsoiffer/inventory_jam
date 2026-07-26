use crate::{
    app::game_is_active,
    projectile::protocol::{PlayerProjectile, ProjectilePosition},
};
use bevy::{
    app::FixedUpdate,
    color::palettes::css::RED,
    log::info,
    prelude::{
        App, AssetServer, Assets, Commands, Component, Entity, Gizmos, Handle, Image,
        IntoScheduleConfigs, Isometry2d, Plugin, Quat, Query, Res, ResMut, Resource, Sprite,
        Startup, TextureAtlas, TextureAtlasLayout, Time, Timer, TimerMode, Transform, UVec2,
        Update, Vec3, Without,
    },
};
use lightyear::core::timeline::LocalTimeline;

pub struct ProjectileRenderPlugin;

impl Plugin for ProjectileRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_projectile_visual_assets);
        app.add_systems(FixedUpdate, debug_projectile_position);
        app.add_systems(
            Update,
            (
                ensure_projectile_sprites,
                sync_projectile_sprites,
                animate_weapon_effect_sprites,
            )
                .chain()
                .run_if(game_is_active),
        );
    }
}

#[derive(Resource)]
struct ProjectileVisualAssets {
    sword_projectile_image: Handle<Image>,
    sword_projectile_layout: Handle<TextureAtlasLayout>,
}

#[derive(Component)]
struct SpriteAnimation {
    first_frame: usize,
    last_frame: usize,
    timer: Timer,
}

fn debug_projectile_position(
    q: Query<(Entity, &ProjectilePosition)>,
    mut gizmos: Gizmos,
    local_timeline: Res<LocalTimeline>,
) {
    for (entity, position) in q.iter() {
        info!(
            "Projectile Pos: {:?} {:?} {:?}",
            local_timeline.tick(),
            position,
            entity
        );
        gizmos.circle_2d(Isometry2d::from_xy(position.x, position.y), 5.0, RED);
    }
}

fn load_projectile_visual_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let projectile_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 48),
        4,
        1,
        None,
        None,
    ));

    commands.insert_resource(ProjectileVisualAssets {
        sword_projectile_image: asset_server.load("weapon/sword/spr_vfx_projectile_sword.png"),
        sword_projectile_layout: projectile_layout,
    });
}

fn ensure_projectile_sprites(
    mut commands: Commands,
    assets: Res<ProjectileVisualAssets>,
    projectiles: Query<(Entity, &PlayerProjectile, &ProjectilePosition), Without<Sprite>>,
) {
    for (entity, projectile, position) in &projectiles {
        let direction = projectile.direction.normalize_or_zero();
        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                assets.sword_projectile_image.clone(),
                TextureAtlas {
                    layout: assets.sword_projectile_layout.clone(),
                    index: 0,
                },
            ),
            SpriteAnimation {
                first_frame: 0,
                last_frame: 3,
                timer: Timer::from_seconds(0.08, TimerMode::Repeating),
            },
            Transform {
                translation: position.extend(8.0),
                rotation: Quat::from_rotation_z(direction.y.atan2(direction.x)),
                scale: Vec3::splat(0.65),
            },
        ));
    }
}

fn sync_projectile_sprites(
    mut projectiles: Query<(&ProjectilePosition, &PlayerProjectile, &mut Transform)>,
) {
    for (position, projectile, mut transform) in &mut projectiles {
        let direction = projectile.direction.normalize_or_zero();
        transform.translation = position.0.extend(8.0);
        transform.rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));
        transform.scale = Vec3::splat(0.65);
    }
}

fn animate_weapon_effect_sprites(
    time: Res<Time>,
    mut sprites: Query<(&mut SpriteAnimation, &mut Sprite)>,
) {
    for (mut animation, mut sprite) in &mut sprites {
        animation.timer.tick(time.delta());

        if !animation.timer.just_finished() {
            continue;
        }

        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };

        atlas.index = if atlas.index >= animation.last_frame {
            animation.first_frame
        } else {
            atlas.index + 1
        };
    }
}
