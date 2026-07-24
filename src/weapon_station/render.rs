use crate::app::game_is_active;
use crate::persistence::CachedPersistentState;
use crate::settings::GameSettings;
use crate::weapon_station::{
    UpgradeStation, UpgradeStationPosition, WeaponStationId, WeaponStationPosition,
};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use lightyear::prelude::Controlled;

pub struct WeaponStationRenderPlugin;

impl Plugin for WeaponStationRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_upgrade_station_assets);
        app.add_systems(
            Update,
            (
                ensure_weapon_station_sprites,
                sync_weapon_station_sprites,
                update_weapon_station_labels,
                ensure_upgrade_station_sprites,
                sync_upgrade_station_sprites,
                update_upgrade_station_label,
            )
                .chain()
                .run_if(game_is_active),
        );
    }
}

#[derive(Component)]
struct WeaponStationSpriteVisual;

#[derive(Component)]
struct WeaponStationLabel;

#[derive(Component)]
struct UpgradeStationSpriteVisual;

#[derive(Component)]
struct UpgradeStationLabel;

#[derive(Resource)]
struct UpgradeStationAssets {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

const WEAPON_STATION_SPRITE_Z: f32 = 2.5;
const WEAPON_STATION_LABEL_OFFSET_Y: f32 = 14.0;
const SAFEZONE_ATLAS_COLUMNS: u32 = 9;
const PROP_ANVIL: UVec2 = UVec2::new(7, 1);

fn load_upgrade_station_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(UpgradeStationAssets {
        image: asset_server.load("world_tiles/spr_tileset_safezone.png"),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(16),
            SAFEZONE_ATLAS_COLUMNS,
            9,
            None,
            None,
        )),
    });
}

fn prop_anvil_atlas_index() -> usize {
    PROP_ANVIL.y as usize * SAFEZONE_ATLAS_COLUMNS as usize + PROP_ANVIL.x as usize
}

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
                WeaponStationLabel,
                Text2d::new(format!("{} Lv0", weapon.name)),
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

fn update_weapon_station_labels(
    settings: Res<GameSettings>,
    local_player: Query<&CachedPersistentState, With<Controlled>>,
    stations: Query<(&WeaponStationId, &Children), With<WeaponStationSpriteVisual>>,
    mut labels: Query<&mut Text2d, With<WeaponStationLabel>>,
) {
    let progress = local_player.single().ok();

    for (station_id, children) in &stations {
        let Some(weapon) = settings.weapons.get(station_id.0) else {
            continue;
        };
        let level = progress
            .map(|state| state.weapon(station_id.0).level)
            .unwrap_or(0);
        let label = format!("{} Lv{level}", weapon.name);
        for child in children.iter() {
            if let Ok(mut text) = labels.get_mut(child)
                && text.0 != label
            {
                text.0 = label.clone();
            }
        }
    }
}

fn ensure_upgrade_station_sprites(
    mut commands: Commands,
    settings: Res<GameSettings>,
    assets: Res<UpgradeStationAssets>,
    stations: Query<
        Entity,
        (
            With<UpgradeStation>,
            With<UpgradeStationPosition>,
            Without<UpgradeStationSpriteVisual>,
        ),
    >,
) {
    for entity in &stations {
        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                assets.image.clone(),
                TextureAtlas {
                    layout: assets.layout.clone(),
                    index: prop_anvil_atlas_index(),
                },
            ),
            Anchor::CENTER,
            Transform {
                translation: Vec3::new(0.0, 0.0, WEAPON_STATION_SPRITE_Z),
                ..default()
            },
            UpgradeStationSpriteVisual,
        ));
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                UpgradeStationLabel,
                Text2d::new(format!(
                    "Upgrade ({})",
                    settings.progression.upgrade_cost_base
                )),
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

fn sync_upgrade_station_sprites(
    mut stations: Query<
        (&UpgradeStationPosition, &mut Transform),
        With<UpgradeStationSpriteVisual>,
    >,
) {
    for (position, mut transform) in &mut stations {
        transform.translation = position.0.extend(WEAPON_STATION_SPRITE_Z);
    }
}

fn update_upgrade_station_label(
    settings: Res<GameSettings>,
    local_player: Query<&CachedPersistentState, With<Controlled>>,
    stations: Query<&Children, With<UpgradeStationSpriteVisual>>,
    mut labels: Query<&mut Text2d, With<UpgradeStationLabel>>,
) {
    let cost = local_player
        .single()
        .ok()
        .and_then(|state| {
            let level = state.weapon(state.equipped_weapon_id).level;
            settings.progression.upgrade_cost(level).ok()
        })
        .unwrap_or(settings.progression.upgrade_cost_base);
    let label = format!("Upgrade ({cost})");

    for children in &stations {
        for child in children.iter() {
            if let Ok(mut text) = labels.get_mut(child)
                && text.0 != label
            {
                text.0 = label.clone();
            }
        }
    }
}
