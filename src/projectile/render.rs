use crate::{
    app::game_is_active,
    projectile::protocol::{PlayerProjectile, ProjectileImpact, ProjectilePosition},
    weapon::protocol::WeaponKind,
};
use bevy::prelude::*;

pub struct ProjectileRenderPlugin;

impl Plugin for ProjectileRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_projectile_visual_assets);
        app.add_systems(
            Update,
            (
                ensure_projectile_sprites,
                sync_projectile_sprites,
                ensure_impact_sprites,
                sync_impact_sprites,
                animate_weapon_effect_sprites,
            )
                .chain()
                .run_if(game_is_active),
        );
    }
}

#[derive(Clone)]
struct AnimatedVisualAsset {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    frame_count: usize,
    frame_seconds: f32,
    scale: f32,
}

#[derive(Resource)]
struct ProjectileVisualAssets {
    sword_projectile: AnimatedVisualAsset,
    sword_impact: AnimatedVisualAsset,
    spear_projectile: AnimatedVisualAsset,
    spear_impact: AnimatedVisualAsset,
    staff_projectile: AnimatedVisualAsset,
    staff_impact: AnimatedVisualAsset,
    bow_projectile: AnimatedVisualAsset,
    bow_impact: AnimatedVisualAsset,
    shuriken_projectile: AnimatedVisualAsset,
    shuriken_impact: AnimatedVisualAsset,
    boomerang_projectile: AnimatedVisualAsset,
    boomerang_impact: AnimatedVisualAsset,
}

#[derive(Component)]
struct SpriteAnimation {
    first_frame: usize,
    last_frame: usize,
    timer: Timer,
}

fn load_projectile_visual_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let sword_projectile_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 48),
        4,
        1,
        None,
        None,
    ));
    let sword_impact_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 48),
        6,
        1,
        None,
        None,
    ));
    let spear_projectile_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 24),
        6,
        1,
        None,
        None,
    ));
    let spear_impact_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 24),
        6,
        1,
        None,
        None,
    ));
    let staff_projectile_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(48, 48),
        8,
        1,
        None,
        None,
    ));
    let staff_impact_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(96, 96),
        7,
        1,
        None,
        None,
    ));
    let bow_projectile_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(16, 16),
        6,
        1,
        None,
        None,
    ));
    let bow_impact_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 16),
        7,
        1,
        None,
        None,
    ));
    let shuriken_projectile_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(16, 16),
        5,
        1,
        None,
        None,
    ));
    let shuriken_impact_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(8, 8),
        5,
        1,
        None,
        None,
    ));
    let boomerang_projectile_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(32, 32),
        16,
        1,
        None,
        None,
    ));
    let boomerang_impact_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(8, 8),
        11,
        1,
        None,
        None,
    ));

    commands.insert_resource(ProjectileVisualAssets {
        sword_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/sword/spr_vfx_projectile_sword.png"),
            layout: sword_projectile_layout,
            frame_count: 4,
            frame_seconds: 0.08,
            scale: 0.65,
        },
        sword_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/sword/spr_vfx_impact_sword.png"),
            layout: sword_impact_layout,
            frame_count: 6,
            frame_seconds: 0.06,
            scale: 0.85,
        },
        spear_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/spear/spr_vfx_projectile_spear.png"),
            layout: spear_projectile_layout,
            frame_count: 6,
            frame_seconds: 0.06,
            scale: 1.0,
        },
        spear_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/spear/spr_vfx_impact_spear.png"),
            layout: spear_impact_layout,
            frame_count: 6,
            frame_seconds: 0.05,
            scale: 1.0,
        },
        staff_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/staff/spr_vfx_projectile_staff.png"),
            layout: staff_projectile_layout,
            frame_count: 8,
            frame_seconds: 0.06,
            scale: 0.75,
        },
        staff_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/staff/spr_vfx_impact_staff.png"),
            layout: staff_impact_layout,
            frame_count: 7,
            frame_seconds: 0.05,
            scale: 0.6,
        },
        bow_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/bow/spr_vfx_projectile_arrow.png"),
            layout: bow_projectile_layout,
            frame_count: 6,
            frame_seconds: 0.07,
            scale: 1.1,
        },
        bow_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/bow/spr_vfx_impact_arrow.png"),
            layout: bow_impact_layout,
            frame_count: 7,
            frame_seconds: 0.05,
            scale: 1.2,
        },
        shuriken_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/shuriken/spr_vfx_projectile_shuriken.png"),
            layout: shuriken_projectile_layout,
            frame_count: 5,
            frame_seconds: 0.05,
            scale: 1.4,
        },
        shuriken_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/shuriken/spr_vfx_impact_shuriken.png"),
            layout: shuriken_impact_layout,
            frame_count: 5,
            frame_seconds: 0.04,
            scale: 2.0,
        },
        boomerang_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/boomerang/spr_vfx_projectile_boomerang.png"),
            layout: boomerang_projectile_layout,
            frame_count: 16,
            frame_seconds: 0.04,
            scale: 1.0,
        },
        boomerang_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/boomerang/spr_vfx_part_boomerang_trail.png"),
            layout: boomerang_impact_layout,
            frame_count: 11,
            frame_seconds: 0.035,
            scale: 2.0,
        },
    });
}

fn ensure_projectile_sprites(
    mut commands: Commands,
    assets: Res<ProjectileVisualAssets>,
    projectiles: Query<(Entity, &PlayerProjectile, &ProjectilePosition), Without<Sprite>>,
) {
    for (entity, projectile, position) in &projectiles {
        let visual = projectile_assets(projectile.weapon, &assets);
        let direction = projectile.direction.normalize_or_zero();

        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                visual.image.clone(),
                TextureAtlas {
                    layout: visual.layout.clone(),
                    index: 0,
                },
            ),
            SpriteAnimation {
                first_frame: 0,
                last_frame: visual.frame_count.saturating_sub(1),
                timer: Timer::from_seconds(visual.frame_seconds, TimerMode::Repeating),
            },
            Transform {
                translation: position.0.extend(8.0),
                rotation: Quat::from_rotation_z(direction.y.atan2(direction.x)),
                scale: Vec3::splat(visual.scale),
            },
        ));
    }
}

fn sync_projectile_sprites(
    assets: Res<ProjectileVisualAssets>,
    mut projectiles: Query<(&ProjectilePosition, &PlayerProjectile, &mut Transform)>,
) {
    for (position, projectile, mut transform) in &mut projectiles {
        let visual = projectile_assets(projectile.weapon, &assets);
        let direction = projectile.direction.normalize_or_zero();
        transform.translation = position.0.extend(8.0);
        transform.rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));
        transform.scale = Vec3::splat(visual.scale);
    }
}

fn ensure_impact_sprites(
    mut commands: Commands,
    assets: Res<ProjectileVisualAssets>,
    impacts: Query<(Entity, &ProjectileImpact), Without<Sprite>>,
) {
    for (entity, impact) in &impacts {
        let visual = impact_assets(impact.weapon, &assets);

        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                visual.image.clone(),
                TextureAtlas {
                    layout: visual.layout.clone(),
                    index: 0,
                },
            ),
            Transform {
                translation: impact.position.extend(9.0),
                scale: Vec3::splat(visual.scale),
                ..default()
            },
            SpriteAnimation {
                first_frame: 0,
                last_frame: visual.frame_count.saturating_sub(1),
                timer: Timer::from_seconds(visual.frame_seconds, TimerMode::Repeating),
            },
        ));
    }
}

fn sync_impact_sprites(
    assets: Res<ProjectileVisualAssets>,
    mut impacts: Query<(&ProjectileImpact, &mut Transform)>,
) {
    for (impact, mut transform) in &mut impacts {
        let visual = impact_assets(impact.weapon, &assets);
        transform.translation = impact.position.extend(9.0);
        transform.scale = Vec3::splat(visual.scale);
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

fn projectile_assets(kind: WeaponKind, assets: &ProjectileVisualAssets) -> &AnimatedVisualAsset {
    match kind {
        WeaponKind::Sword => &assets.sword_projectile,
        WeaponKind::Spear => &assets.spear_projectile,
        WeaponKind::Staff => &assets.staff_projectile,
        WeaponKind::Bow => &assets.bow_projectile,
        WeaponKind::Shuriken => &assets.shuriken_projectile,
        WeaponKind::Boomerang => &assets.boomerang_projectile,
    }
}

fn impact_assets(kind: WeaponKind, assets: &ProjectileVisualAssets) -> &AnimatedVisualAsset {
    match kind {
        WeaponKind::Sword => &assets.sword_impact,
        WeaponKind::Spear => &assets.spear_impact,
        WeaponKind::Staff => &assets.staff_impact,
        WeaponKind::Bow => &assets.bow_impact,
        WeaponKind::Shuriken => &assets.shuriken_impact,
        WeaponKind::Boomerang => &assets.boomerang_impact,
    }
}
