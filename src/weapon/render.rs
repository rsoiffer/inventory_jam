use bevy::prelude::*;
use lightyear::prelude::{Interpolated, Predicted, Replicate};

use crate::persistence::CachedPersistentState;
use crate::player::{PlayerAimDirection, PlayerPosition};
use crate::{app::game_is_active, settings::GameSettings, weapon::protocol::WeaponId};

#[cfg(feature = "gui")]
pub struct WeaponRenderPlugin;

#[cfg(feature = "gui")]
impl Plugin for WeaponRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (ensure_weapon_sprites, sync_weapon_sprites)
                .chain()
                .run_if(game_is_active),
        );
    }
}

#[derive(Component, Clone, Copy, Debug)]
struct WeaponSprite {
    id: WeaponId,
}

type VisiblePlayer = (
    Or<(With<Predicted>, With<Interpolated>, With<Replicate>)>,
    Without<WeaponSprite>,
);

fn ensure_weapon_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    players: Query<(Entity, &CachedPersistentState), VisiblePlayer>,
) {
    for (entity, cache) in &players {
        let Some(weapon) = settings.weapons.get(cache.equipped_weapon_id) else {
            continue;
        };
        commands.entity(entity).insert((
            Sprite::from_image(asset_server.load(weapon.icon.clone())),
            Transform::default(),
            WeaponSprite {
                id: cache.equipped_weapon_id,
            },
        ));
    }
}

fn sync_weapon_sprites(
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    mut players: Query<(
        &PlayerPosition,
        &PlayerAimDirection,
        &CachedPersistentState,
        &mut WeaponSprite,
        &mut Sprite,
        &mut Transform,
    )>,
) {
    for (position, aim, cache, mut visual, mut sprite, mut transform) in &mut players {
        let direction = aim.0.normalize_or_zero();
        if direction == Vec2::ZERO {
            continue;
        }

        if visual.id != cache.equipped_weapon_id {
            let Some(weapon) = settings.weapons.get(cache.equipped_weapon_id) else {
                continue;
            };
            sprite.image = asset_server.load(weapon.icon.clone());
            visual.id = cache.equipped_weapon_id;
        }

        let visual_position = position.0 + direction * settings.player_visual.held_weapon_offset;
        transform.translation = visual_position.extend(10.0);
        transform.rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));
        transform.scale = Vec3::ONE;
        sprite.flip_y = direction.x < 0.0;
    }
}
