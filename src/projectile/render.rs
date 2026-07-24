use crate::{
    app::game_is_active,
    projectile::protocol::{ProjectileBuffer, ProjectileSlot, ProjectileState},
    weapon::protocol::WeaponKind,
};
use bevy::prelude::*;
use lightyear::{interpolation::Interpolated, prediction::Predicted, prelude::Replicate};

pub struct ProjectileRenderPlugin;

impl Plugin for ProjectileRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_projectile_visual_assets);
        app.add_systems(
            Update,
            (sync_buffer_visuals, animate_weapon_effect_sprites)
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
    by_weapon: [[AnimatedVisualAsset; 2]; 6],
}

impl ProjectileVisualAssets {
    fn get(&self, kind: WeaponKind, impact: bool) -> &AnimatedVisualAsset {
        &self.by_weapon[kind as usize][impact as usize]
    }
}

#[derive(Component)]
struct SpriteAnimation {
    last_frame: usize,
    timer: Timer,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct ProjectileVisualKey {
    owner: Entity,
    slot_index: u16,
    generation: u32,
    is_impact: bool,
}

fn load_anim(
    asset_server: &AssetServer,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
    path: &'static str,
    cell: UVec2,
    frames: u32,
    frame_seconds: f32,
) -> AnimatedVisualAsset {
    AnimatedVisualAsset {
        image: asset_server.load(path),
        layout: atlas_layouts.add(TextureAtlasLayout::from_grid(cell, frames, 1, None, None)),
        frame_count: frames as usize,
        frame_seconds,
    }
}

fn load_projectile_visual_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let mut load = |path, cell, frames, secs| {
        load_anim(&asset_server, &mut atlas_layouts, path, cell, frames, secs)
    };
    // Index order matches WeaponKind discriminant: projectile then impact.
    commands.insert_resource(ProjectileVisualAssets {
        by_weapon: [
            [
                load(
                    "weapon/sword/spr_vfx_projectile_sword.png",
                    UVec2::new(24, 48),
                    4,
                    0.08,
                ),
                load(
                    "weapon/sword/spr_vfx_impact_sword.png",
                    UVec2::new(24, 48),
                    6,
                    0.06,
                ),
            ],
            [
                load(
                    "weapon/spear/spr_vfx_projectile_spear.png",
                    UVec2::new(24, 24),
                    6,
                    0.06,
                ),
                load(
                    "weapon/spear/spr_vfx_impact_spear.png",
                    UVec2::new(24, 24),
                    6,
                    0.05,
                ),
            ],
            [
                load(
                    "weapon/staff/spr_vfx_projectile_staff.png",
                    UVec2::new(48, 48),
                    8,
                    0.06,
                ),
                load(
                    "weapon/staff/spr_vfx_impact_staff.png",
                    UVec2::new(96, 96),
                    7,
                    0.05,
                ),
            ],
            [
                load(
                    "weapon/bow/spr_vfx_projectile_arrow.png",
                    UVec2::new(16, 16),
                    6,
                    0.07,
                ),
                load(
                    "weapon/bow/spr_vfx_impact_arrow.png",
                    UVec2::new(24, 16),
                    7,
                    0.05,
                ),
            ],
            [
                load(
                    "weapon/shuriken/spr_vfx_projectile_shuriken.png",
                    UVec2::new(16, 16),
                    5,
                    0.05,
                ),
                load(
                    "weapon/shuriken/spr_vfx_impact_shuriken.png",
                    UVec2::new(8, 8),
                    5,
                    0.04,
                ),
            ],
            [
                load(
                    "weapon/boomerang/spr_vfx_projectile_boomerang.png",
                    UVec2::new(32, 32),
                    16,
                    0.04,
                ),
                load(
                    "weapon/boomerang/spr_vfx_part_boomerang_trail.png",
                    UVec2::new(8, 8),
                    11,
                    0.035,
                ),
            ],
        ],
    });
}

fn sync_buffer_visuals(
    mut commands: Commands,
    assets: Res<ProjectileVisualAssets>,
    buffers: Query<
        (Entity, &ProjectileBuffer),
        Or<(With<Predicted>, With<Interpolated>, With<Replicate>)>,
    >,
    mut visuals: Query<(Entity, &ProjectileVisualKey, &mut Transform)>,
) {
    let mut live_keys = Vec::new();

    for (owner, buffer) in &buffers {
        for (slot_index, slot) in buffer.slots.iter().enumerate() {
            let ProjectileSlot::Active {
                generation,
                weapon,
                position,
                velocity,
                state,
            } = slot
            else {
                continue;
            };
            let is_impact = matches!(state, ProjectileState::Impact { .. });
            let key = ProjectileVisualKey {
                owner,
                slot_index: slot_index as u16,
                generation: *generation,
                is_impact,
            };
            live_keys.push(key);

            let direction = velocity.normalize_or_zero();
            let translation = position.extend(if is_impact { 9.0 } else { 8.0 });
            let rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));

            if let Some((_, _, mut transform)) =
                visuals.iter_mut().find(|(_, existing, _)| **existing == key)
            {
                transform.translation = translation;
                transform.rotation = rotation;
                transform.scale = Vec3::ONE;
                continue;
            }

            let visual = assets.get(*weapon, is_impact);
            commands.spawn((
                key,
                Name::new(if is_impact {
                    "Projectile Impact Visual"
                } else {
                    "Projectile Visual"
                }),
                Sprite::from_atlas_image(
                    visual.image.clone(),
                    TextureAtlas {
                        layout: visual.layout.clone(),
                        index: 0,
                    },
                ),
                SpriteAnimation {
                    last_frame: visual.frame_count.saturating_sub(1),
                    timer: Timer::from_seconds(visual.frame_seconds, TimerMode::Repeating),
                },
                Transform {
                    translation,
                    rotation,
                    ..default()
                },
            ));
        }
    }

    for (entity, key, _) in &visuals {
        if !live_keys.contains(key) {
            commands.entity(entity).despawn();
        }
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
            0
        } else {
            atlas.index + 1
        };
    }
}
