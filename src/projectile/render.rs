use crate::{
    app::game_is_active,
    projectile::protocol::{ProjectileBuffer, ProjectileSlot, ProjectileSource, ProjectileState},
    settings::{GameSettings, WeaponSpriteSheetSettings},
    weapon::protocol::WeaponId,
};
use bevy::prelude::*;
use lightyear::{interpolation::Interpolated, prediction::Predicted, prelude::Replicate};
use std::collections::HashMap;

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
    by_weapon: HashMap<WeaponId, [AnimatedVisualAsset; 2]>,
    enemy: [AnimatedVisualAsset; 2],
    grand_champion: [AnimatedVisualAsset; 2],
}

impl ProjectileVisualAssets {
    fn get(&self, source: ProjectileSource, impact: bool) -> Option<&AnimatedVisualAsset> {
        match source {
            ProjectileSource::Weapon(id) => {
                self.by_weapon.get(&id).map(|pair| &pair[impact as usize])
            }
            ProjectileSource::Enemy => Some(&self.enemy[impact as usize]),
            ProjectileSource::GrandChampion => Some(&self.grand_champion[impact as usize]),
        }
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
    sheet: &WeaponSpriteSheetSettings,
) -> AnimatedVisualAsset {
    AnimatedVisualAsset {
        image: asset_server.load(sheet.path.clone()),
        layout: atlas_layouts.add(TextureAtlasLayout::from_grid(
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

fn load_projectile_visual_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    settings: Res<GameSettings>,
) {
    let by_weapon = settings
        .weapons
        .0
        .iter()
        .map(|weapon| {
            (
                weapon.id,
                [
                    load_anim(&asset_server, &mut atlas_layouts, &weapon.projectile),
                    load_anim(&asset_server, &mut atlas_layouts, &weapon.impact),
                ],
            )
        })
        .collect();
    let enemy = [
        load_anim(
            &asset_server,
            &mut atlas_layouts,
            &settings.enemy.projectile,
        ),
        load_anim(&asset_server, &mut atlas_layouts, &settings.enemy.impact),
    ];
    let grand_champion = enemy.clone();
    commands.insert_resource(ProjectileVisualAssets {
        by_weapon,
        enemy,
        grand_champion,
    });
}

fn sync_buffer_visuals(
    mut commands: Commands,
    assets: Res<ProjectileVisualAssets>,
    settings: Res<GameSettings>,
    buffers: Query<
        (Entity, &ProjectileBuffer),
        Or<(With<Predicted>, With<Interpolated>, With<Replicate>)>,
    >,
    mut visuals: Query<(Entity, &ProjectileVisualKey, &mut Transform)>,
) {
    let mut live_keys = Vec::new();
    let tint = settings.post_processing.projectile_tint;

    for (owner, buffer) in &buffers {
        for (slot_index, slot) in buffer.slots.iter().enumerate() {
            let ProjectileSlot::Active {
                generation,
                source,
                position,
                velocity,
                state,
                ..
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

            if let Some((_, _, mut transform)) = visuals
                .iter_mut()
                .find(|(_, existing, _)| **existing == key)
            {
                transform.translation = translation;
                transform.rotation = rotation;
                transform.scale = Vec3::ONE;
                continue;
            }

            let Some(visual) = assets.get(*source, is_impact) else {
                continue;
            };
            let mut sprite = Sprite::from_atlas_image(
                visual.image.clone(),
                TextureAtlas {
                    layout: visual.layout.clone(),
                    index: 0,
                },
            );
            sprite.color = tint;
            commands.spawn((
                key,
                Name::new(if is_impact {
                    "Projectile Impact Visual"
                } else {
                    "Projectile Visual"
                }),
                sprite,
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
