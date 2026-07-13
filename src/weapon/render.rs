use bevy::prelude::*;
use lightyear::prelude::{Interpolated, Predicted, Replicate};

use crate::{
    app::game_is_active,
    protocol::{PlayerAimDirection, PlayerPosition},
    weapon::protocol::{EquippedWeapon, WeaponKind},
};

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (ensure_weapon_sprites, sync_weapon_sprites)
            .chain()
            .run_if(game_is_active),
    );
}

#[derive(Component, Clone, Copy, Debug)]
struct WeaponSprite {
    kind: WeaponKind,
}

type VisiblePlayer = (
    Or<(With<Predicted>, With<Interpolated>, With<Replicate>)>,
    Without<WeaponSprite>,
);

fn ensure_weapon_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    players: Query<(Entity, &EquippedWeapon), VisiblePlayer>,
) {
    for (entity, equipped_weapon) in &players {
        commands.entity(entity).insert((
            Sprite::from_image(asset_server.load(weapon_sprite_path(equipped_weapon.kind))),
            Transform::default(),
            WeaponSprite {
                kind: equipped_weapon.kind,
            },
        ));
    }
}

fn sync_weapon_sprites(
    asset_server: Res<AssetServer>,
    mut players: Query<(
        &PlayerPosition,
        &PlayerAimDirection,
        &EquippedWeapon,
        &mut WeaponSprite,
        &mut Sprite,
        &mut Transform,
    )>,
) {
    const HELD_WEAPON_OFFSET: f32 = 34.0;
    const HELD_WEAPON_SCALE: f32 = 1.4;

    for (position, aim, equipped_weapon, mut visual, mut sprite, mut transform) in &mut players {
        let direction = aim.0.normalize_or_zero();
        if direction == Vec2::ZERO {
            continue;
        }

        if visual.kind != equipped_weapon.kind {
            sprite.image = asset_server.load(weapon_sprite_path(equipped_weapon.kind));
            visual.kind = equipped_weapon.kind;
        }

        let visual_position = position.0 + direction * HELD_WEAPON_OFFSET;
        transform.translation = visual_position.extend(10.0);
        transform.rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));
        transform.scale = Vec3::splat(HELD_WEAPON_SCALE);
    }
}

fn weapon_sprite_path(kind: WeaponKind) -> &'static str {
    match kind {
        WeaponKind::Sword => "weapon/sword/spr_icon_sword.png",
        // Placeholder mappings until these weapon assets exist.
        WeaponKind::Spear => "weapon/sword/spr_icon_sword.png",
        WeaponKind::Staff => "weapon/sword/spr_icon_sword.png",
    }
}
