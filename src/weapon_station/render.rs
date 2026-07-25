use crate::app::game_is_active;
use crate::persistence::CachedPersistentState;
use crate::settings::GameSettings;
use crate::weapon_station::{
    StationPosition, UpgradeStationKind, WeaponStationId, upgrade_station_label,
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
                ensure_upgrade_station_sprites,
                sync_station_sprites,
                update_station_labels,
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

const STATION_SPRITE_Z: f32 = 2.5;
const STATION_LABEL_OFFSET_Y: f32 = 14.0;
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

fn station_label_bundle(text: String) -> impl Bundle {
    (
        Text2d::new(text),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Anchor::BOTTOM_CENTER,
        Transform::from_translation(Vec3::new(0.0, STATION_LABEL_OFFSET_Y, 0.1))
            .with_scale(Vec3::splat(0.48)),
    )
}

fn ensure_weapon_station_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    stations: Query<
        (Entity, &WeaponStationId),
        (With<StationPosition>, Without<WeaponStationSpriteVisual>),
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
                translation: Vec3::new(0.0, 0.0, STATION_SPRITE_Z),
                ..default()
            },
            WeaponStationSpriteVisual,
        ));
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                WeaponStationLabel,
                station_label_bundle(format!("{} Lv0", weapon.name)),
            ));
        });
    }
}

fn ensure_upgrade_station_sprites(
    mut commands: Commands,
    settings: Res<GameSettings>,
    assets: Res<UpgradeStationAssets>,
    stations: Query<
        (Entity, &UpgradeStationKind),
        (With<StationPosition>, Without<UpgradeStationSpriteVisual>),
    >,
) {
    for (entity, kind) in &stations {
        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                assets.image.clone(),
                TextureAtlas {
                    layout: assets.layout.clone(),
                    index: PROP_ANVIL.y as usize * SAFEZONE_ATLAS_COLUMNS as usize
                        + PROP_ANVIL.x as usize,
                },
            ),
            Anchor::CENTER,
            Transform {
                translation: Vec3::new(0.0, 0.0, STATION_SPRITE_Z),
                ..default()
            },
            UpgradeStationSpriteVisual,
        ));
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                UpgradeStationLabel,
                station_label_bundle(upgrade_station_label(
                    kind.0,
                    0,
                    settings.progression.upgrade_cost_base,
                )),
            ));
        });
    }
}

fn sync_station_sprites(
    mut stations: Query<
        (&StationPosition, &mut Transform),
        Or<(
            With<WeaponStationSpriteVisual>,
            With<UpgradeStationSpriteVisual>,
        )>,
    >,
) {
    for (position, mut transform) in &mut stations {
        transform.translation = position.0.extend(STATION_SPRITE_Z);
    }
}

fn update_station_labels(
    settings: Res<GameSettings>,
    local_player: Query<&CachedPersistentState, With<Controlled>>,
    weapon_stations: Query<(&WeaponStationId, &Children), With<WeaponStationSpriteVisual>>,
    upgrade_stations: Query<(&UpgradeStationKind, &Children), With<UpgradeStationSpriteVisual>>,
    mut weapon_labels: Query<&mut Text2d, (With<WeaponStationLabel>, Without<UpgradeStationLabel>)>,
    mut upgrade_labels: Query<
        &mut Text2d,
        (With<UpgradeStationLabel>, Without<WeaponStationLabel>),
    >,
) {
    let state = local_player.single().ok();

    for (station_id, children) in &weapon_stations {
        let Some(weapon) = settings.weapons.get(station_id.0) else {
            continue;
        };
        let level = state
            .map(|cache| cache.weapon(station_id.0).total_level())
            .unwrap_or(0);
        let label = format!("{} Lv{level}", weapon.name);
        for child in children.iter() {
            if let Ok(mut text) = weapon_labels.get_mut(child)
                && text.0 != label
            {
                text.0 = label.clone();
            }
        }
    }

    for (kind, children) in &upgrade_stations {
        let (level, cost) = state
            .map(|cache| {
                let level = cache.weapon(cache.equipped_weapon_id).level_for(kind.0);
                let cost = settings
                    .progression
                    .upgrade_cost(level)
                    .unwrap_or(settings.progression.upgrade_cost_base);
                (level, cost)
            })
            .unwrap_or((0, settings.progression.upgrade_cost_base));
        let upgrade_label = upgrade_station_label(kind.0, level, cost);
        for child in children.iter() {
            if let Ok(mut text) = upgrade_labels.get_mut(child)
                && text.0 != upgrade_label
            {
                text.0 = upgrade_label.clone();
            }
        }
    }
}
