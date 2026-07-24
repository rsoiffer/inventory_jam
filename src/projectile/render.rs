use crate::{
    app::game_is_active,
    projectile::protocol::{
        PlayerProjectile, PredictedHitHidden, PredictedProjectileImpact, ProjectileImpact,
        ProjectilePosition,
    },
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
                ensure_predicted_impact_sprites,
                despawn_confirmed_predicted_impacts,
                sync_impact_sprites,
                sync_predicted_impact_sprites,
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

#[derive(Component, Clone)]
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
        },
        sword_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/sword/spr_vfx_impact_sword.png"),
            layout: sword_impact_layout,
            frame_count: 6,
            frame_seconds: 0.06,
        },
        spear_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/spear/spr_vfx_projectile_spear.png"),
            layout: spear_projectile_layout,
            frame_count: 6,
            frame_seconds: 0.06,
        },
        spear_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/spear/spr_vfx_impact_spear.png"),
            layout: spear_impact_layout,
            frame_count: 6,
            frame_seconds: 0.05,
        },
        staff_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/staff/spr_vfx_projectile_staff.png"),
            layout: staff_projectile_layout,
            frame_count: 8,
            frame_seconds: 0.06,
        },
        staff_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/staff/spr_vfx_impact_staff.png"),
            layout: staff_impact_layout,
            frame_count: 7,
            frame_seconds: 0.05,
        },
        bow_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/bow/spr_vfx_projectile_arrow.png"),
            layout: bow_projectile_layout,
            frame_count: 6,
            frame_seconds: 0.07,
        },
        bow_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/bow/spr_vfx_impact_arrow.png"),
            layout: bow_impact_layout,
            frame_count: 7,
            frame_seconds: 0.05,
        },
        shuriken_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/shuriken/spr_vfx_projectile_shuriken.png"),
            layout: shuriken_projectile_layout,
            frame_count: 5,
            frame_seconds: 0.05,
        },
        shuriken_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/shuriken/spr_vfx_impact_shuriken.png"),
            layout: shuriken_impact_layout,
            frame_count: 5,
            frame_seconds: 0.04,
        },
        boomerang_projectile: AnimatedVisualAsset {
            image: asset_server.load("weapon/boomerang/spr_vfx_projectile_boomerang.png"),
            layout: boomerang_projectile_layout,
            frame_count: 16,
            frame_seconds: 0.04,
        },
        boomerang_impact: AnimatedVisualAsset {
            image: asset_server.load("weapon/boomerang/spr_vfx_part_boomerang_trail.png"),
            layout: boomerang_impact_layout,
            frame_count: 11,
            frame_seconds: 0.035,
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
                ..default()
            },
        ));
    }
}

fn sync_projectile_sprites(
    mut projectiles: Query<(
        &ProjectilePosition,
        &PlayerProjectile,
        &mut Transform,
        &mut Sprite,
        Option<&PredictedHitHidden>,
    )>,
) {
    for (position, projectile, mut transform, mut sprite, hidden) in &mut projectiles {
        let direction = projectile.direction.normalize_or_zero();
        transform.translation = position.0.extend(8.0);
        transform.rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));
        transform.scale = Vec3::ONE;
        sprite.color = if hidden.is_some() {
            Color::NONE
        } else {
            Color::WHITE
        };
    }
}

fn ensure_impact_sprites(
    mut commands: Commands,
    assets: Res<ProjectileVisualAssets>,
    impacts: Query<(Entity, &ProjectileImpact), Without<Sprite>>,
    predicted: Query<(&PredictedProjectileImpact, &Sprite, &SpriteAnimation)>,
) {
    for (entity, impact) in &impacts {
        let carried = predicted
            .iter()
            .find(|(predicted_impact, _, _)| predicted_impact.shot_id == impact.shot_id)
            .map(|(_, sprite, animation)| {
                (
                    sprite
                        .texture_atlas
                        .as_ref()
                        .map(|atlas| atlas.index)
                        .unwrap_or(0),
                    animation.clone(),
                )
            });

        insert_impact_sprite(
            &mut commands,
            entity,
            impact.weapon,
            impact.direction,
            impact.position,
            &assets,
            carried,
        );
    }
}

fn ensure_predicted_impact_sprites(
    mut commands: Commands,
    assets: Res<ProjectileVisualAssets>,
    impacts: Query<(Entity, &PredictedProjectileImpact), Without<Sprite>>,
) {
    for (entity, impact) in &impacts {
        insert_impact_sprite(
            &mut commands,
            entity,
            impact.weapon,
            impact.direction,
            impact.position,
            &assets,
            None,
        );
    }
}

fn insert_impact_sprite(
    commands: &mut Commands,
    entity: Entity,
    weapon: WeaponKind,
    direction: Vec2,
    position: Vec2,
    assets: &ProjectileVisualAssets,
    carried_animation: Option<(usize, SpriteAnimation)>,
) {
    let visual = impact_assets(weapon, assets);
    let direction = direction.normalize_or_zero();
    let (atlas_index, animation) = carried_animation.unwrap_or_else(|| {
        (
            0,
            SpriteAnimation {
                first_frame: 0,
                last_frame: visual.frame_count.saturating_sub(1),
                timer: Timer::from_seconds(visual.frame_seconds, TimerMode::Repeating),
            },
        )
    });

    commands.entity(entity).insert((
        Sprite::from_atlas_image(
            visual.image.clone(),
            TextureAtlas {
                layout: visual.layout.clone(),
                index: atlas_index,
            },
        ),
        Transform {
            translation: position.extend(9.0),
            rotation: Quat::from_rotation_z(direction.y.atan2(direction.x)),
            ..default()
        },
        animation,
    ));
}

/// Drop local predicted impacts once the matching authoritative impact has a
/// sprite, after animation progress has been copied over.
fn despawn_confirmed_predicted_impacts(
    mut commands: Commands,
    server_impacts: Query<&ProjectileImpact, With<Sprite>>,
    predicted_impacts: Query<(Entity, &PredictedProjectileImpact)>,
) {
    for (entity, predicted) in &predicted_impacts {
        if server_impacts
            .iter()
            .any(|impact| impact.shot_id == predicted.shot_id)
        {
            commands.entity(entity).despawn();
        }
    }
}

fn sync_impact_sprites(mut impacts: Query<(&ProjectileImpact, &mut Transform)>) {
    for (impact, mut transform) in &mut impacts {
        let direction = impact.direction.normalize_or_zero();
        transform.translation = impact.position.extend(9.0);
        transform.rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));
        transform.scale = Vec3::ONE;
    }
}

fn sync_predicted_impact_sprites(mut impacts: Query<(&PredictedProjectileImpact, &mut Transform)>) {
    for (impact, mut transform) in &mut impacts {
        let direction = impact.direction.normalize_or_zero();
        transform.translation = impact.position.extend(9.0);
        transform.rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));
        transform.scale = Vec3::ONE;
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
