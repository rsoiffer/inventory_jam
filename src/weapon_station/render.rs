use crate::app::game_is_active;
use crate::settings::GameSettings;
use crate::weapon_station::{WeaponStationId, WeaponStationPosition};
use bevy::prelude::*;
use bevy::sprite::Anchor;

pub struct WeaponStationRenderPlugin;

impl Plugin for WeaponStationRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (ensure_weapon_station_sprites, sync_weapon_station_sprites)
                .chain()
                .run_if(game_is_active),
        );
    }
}

#[derive(Component)]
struct WeaponStationSpriteVisual;

const WEAPON_STATION_SPRITE_Z: f32 = 2.5;
const WEAPON_STATION_LABEL_OFFSET_Y: f32 = 14.0;

fn ensure_weapon_station_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    stations: Query<
        (Entity, &WeaponStationId),
        (
            With<WeaponStationPosition>,
            Without<WeaponStationSpriteVisual>,
        ),
    >,
) {
    for (entity, station_id) in &stations {
        let Some(weapon) = settings.weapons.get(station_id.0) else {
            continue;
        };
        commands.entity(entity).insert((
            Sprite::from_image(asset_server.load(weapon.icon.clone())),
            Anchor::CENTER,
            Transform {
                translation: Vec3::new(0.0, 0.0, WEAPON_STATION_SPRITE_Z),
                ..default()
            },
            WeaponStationSpriteVisual,
        ));
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Text2d::new(weapon.name.clone()),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Anchor::BOTTOM_CENTER,
                Transform::from_translation(Vec3::new(0.0, WEAPON_STATION_LABEL_OFFSET_Y, 0.1))
                    .with_scale(Vec3::splat(0.48)),
            ));
        });
    }
}

fn sync_weapon_station_sprites(
    mut stations: Query<(&WeaponStationPosition, &mut Transform), With<WeaponStationSpriteVisual>>,
) {
    for (position, mut transform) in &mut stations {
        transform.translation = position.0.extend(WEAPON_STATION_SPRITE_Z);
    }
}
