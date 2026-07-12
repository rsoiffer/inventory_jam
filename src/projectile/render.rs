use bevy::prelude::*;

use crate::{
    projectile::{
        protocol::{PlayerProjectile, ProjectileImpact},
        shared::{ClientImpactVisual, ClientProjectileVisual, ImpactPosition, ProjectilePosition},
    },
    weapon::protocol::WeaponKind,
};

pub(super) fn register(app: &mut App) {
    app.add_systems(Startup, load_projectile_visual_assets);

    app.add_systems(
        Update,
        (
            ensure_projectile_sprites,
            sync_projectile_sprites,
            cleanup_projectile_sprites,
            ensure_impact_sprites,
            sync_impact_sprites,
            animate_weapon_effect_sprites,
        )
            .chain(),
    );
}

#[derive(Resource)]
struct ProjectileVisualAssets {
    sword_projectile_image: Handle<Image>,
    sword_projectile_layout: Handle<TextureAtlasLayout>,
    sword_impact_image: Handle<Image>,
    sword_impact_layout: Handle<TextureAtlasLayout>,
}

#[derive(Component)]
struct ProjectileSprite;

#[derive(Component)]
struct ImpactSprite;

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
    let projectile_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 48),
        4,
        1,
        None,
        None,
    ));

    let impact_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 48),
        6,
        1,
        None,
        None,
    ));

    commands.insert_resource(ProjectileVisualAssets {
        sword_projectile_image: asset_server.load("weapon/sword/spr_vfx_projectile_sword.png"),
        sword_projectile_layout: projectile_layout,
        sword_impact_image: asset_server.load("weapon/sword/spr_vfx_impact_sword.png"),
        sword_impact_layout: impact_layout,
    });
}

fn ensure_projectile_sprites(
    mut commands: Commands,
    assets: Res<ProjectileVisualAssets>,
    projectiles: Query<
        (Entity, &PlayerProjectile),
        (With<ClientProjectileVisual>, Without<ProjectileSprite>),
    >,
) {
    for (entity, projectile) in &projectiles {
        let (image, layout) = projectile_assets(projectile.weapon, &assets);

        commands.entity(entity).insert((
            Sprite::from_atlas_image(image, TextureAtlas { layout, index: 0 }),
            Transform::default(),
            ProjectileSprite,
            SpriteAnimation {
                first_frame: 0,
                last_frame: 3,
                timer: Timer::from_seconds(0.08, TimerMode::Repeating),
            },
        ));
    }
}

fn sync_projectile_sprites(
    mut projectiles: Query<
        (&ProjectilePosition, &PlayerProjectile, &mut Transform),
        With<ProjectileSprite>,
    >,
) {
    for (position, projectile, mut transform) in &mut projectiles {
        let direction = projectile.direction.normalize_or_zero();
        transform.translation = position.0.extend(8.0);
        transform.rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));
        transform.scale = Vec3::splat(0.65);
    }
}

fn cleanup_projectile_sprites(
    mut commands: Commands,
    projectiles: Query<Entity, (With<ProjectileSprite>, Without<ClientProjectileVisual>)>,
) {
    for entity in &projectiles {
        commands
            .entity(entity)
            .remove::<(ProjectileSprite, SpriteAnimation, Sprite, Transform)>();
    }
}

fn ensure_impact_sprites(
    mut commands: Commands,
    assets: Res<ProjectileVisualAssets>,
    impacts: Query<(Entity, &ProjectileImpact), (With<ClientImpactVisual>, Without<ImpactSprite>)>,
) {
    for (entity, impact) in &impacts {
        let (image, layout) = impact_assets(impact.weapon, &assets);

        commands.entity(entity).insert((
            Sprite::from_atlas_image(image, TextureAtlas { layout, index: 0 }),
            Transform::default(),
            ImpactSprite,
            SpriteAnimation {
                first_frame: 0,
                last_frame: 5,
                timer: Timer::from_seconds(0.06, TimerMode::Repeating),
            },
        ));
    }
}

fn sync_impact_sprites(mut impacts: Query<(&ImpactPosition, &mut Transform), With<ImpactSprite>>) {
    for (position, mut transform) in &mut impacts {
        transform.translation = position.0.extend(9.0);
        transform.scale = Vec3::splat(0.85);
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

fn projectile_assets(
    _kind: WeaponKind,
    assets: &ProjectileVisualAssets,
) -> (Handle<Image>, Handle<TextureAtlasLayout>) {
    (
        assets.sword_projectile_image.clone(),
        assets.sword_projectile_layout.clone(),
    )
}

fn impact_assets(
    _kind: WeaponKind,
    assets: &ProjectileVisualAssets,
) -> (Handle<Image>, Handle<TextureAtlasLayout>) {
    (
        assets.sword_impact_image.clone(),
        assets.sword_impact_layout.clone(),
    )
}
