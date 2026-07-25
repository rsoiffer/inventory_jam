use crate::settings::{GameSettings, SafezoneFloorRect};
use bevy::camera::Camera2d;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct WorldRenderPlugin;

impl Plugin for WorldRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadedWorldTiles>();
        app.add_systems(
            Startup,
            (load_world_tile_assets, spawn_pit_decorations).chain(),
        );
        app.add_systems(Update, stream_world_tiles);
        #[cfg(feature = "dev")]
        app.add_systems(Update, draw_world_boundaries);
    }
}

#[derive(Component)]
struct WorldTile;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TileKey {
    x: i32,
    y: i32,
}

#[derive(Resource)]
struct WorldTileAssets {
    arena_image: Handle<Image>,
    arena_layout: Handle<TextureAtlasLayout>,
    safezone_image: Handle<Image>,
    safezone_layout: Handle<TextureAtlasLayout>,
}

#[derive(Resource, Default)]
struct LoadedWorldTiles {
    entities: HashMap<TileKey, Entity>,
    last_camera_tile: Option<IVec2>,
}

const PIT_BACKGROUND_Z: f32 = -2.0;
const PIT_FLOOR_Z: f32 = -1.0;
const PIT_WALL_Z: f32 = 0.0;
const PIT_PROP_Z: f32 = 1.0;

#[derive(Clone, Copy)]
struct AtlasTile {
    x: u32,
    y: u32,
    columns: u32,
}

impl AtlasTile {
    fn from_uvec2(tile: UVec2, columns: u32) -> Self {
        Self {
            x: tile.x,
            y: tile.y,
            columns,
        }
    }

    fn atlas_index(self) -> usize {
        self.y as usize * self.columns as usize + self.x as usize
    }

    fn offset(self, x: i32, y: i32) -> Self {
        Self {
            x: (self.x as i32 + x) as u32,
            y: (self.y as i32 + y) as u32,
            columns: self.columns,
        }
    }
}

fn load_world_tile_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    settings: Res<GameSettings>,
) {
    let arena = &settings.world.arena_tileset;
    let safezone = &settings.world.safezone.tileset;
    commands.insert_resource(WorldTileAssets {
        arena_image: asset_server.load(arena.path.clone()),
        arena_layout: layouts.add(TextureAtlasLayout::from_grid(
            arena.cell,
            arena.columns,
            arena.rows,
            None,
            None,
        )),
        safezone_image: asset_server.load(safezone.path.clone()),
        safezone_layout: layouts.add(TextureAtlasLayout::from_grid(
            safezone.cell,
            safezone.columns,
            safezone.rows,
            None,
            None,
        )),
    });
}

fn stream_world_tiles(
    mut commands: Commands,
    assets: Option<Res<WorldTileAssets>>,
    cameras: Query<&GlobalTransform, With<Camera2d>>,
    mut loaded: ResMut<LoadedWorldTiles>,
    settings: Res<GameSettings>,
) {
    let Some(assets) = assets else {
        return;
    };

    let Some(camera_transform) = cameras.iter().next() else {
        return;
    };

    let camera_position = camera_transform.translation().truncate();
    let camera_tile = IVec2::new(
        (camera_position.x / settings.world.tile_pixel_size).floor() as i32,
        (camera_position.y / settings.world.tile_pixel_size).floor() as i32,
    );

    if loaded.last_camera_tile == Some(camera_tile) {
        return;
    }
    loaded.last_camera_tile = Some(camera_tile);

    let mut desired = HashSet::new();

    for y in (camera_tile.y - settings.world.stream_half_height)
        ..=(camera_tile.y + settings.world.stream_half_height)
    {
        for x in (camera_tile.x - settings.world.stream_half_width)
            ..=(camera_tile.x + settings.world.stream_half_width)
        {
            let world_position = Vec2::new(
                (x as f32 + 0.5) * settings.world.tile_pixel_size,
                (y as f32 + 0.5) * settings.world.tile_pixel_size,
            );

            if !settings.world.arena_bounds().contains(world_position) {
                continue;
            }

            let key = TileKey { x, y };
            desired.insert(key);

            if loaded.entities.contains_key(&key) {
                continue;
            }

            let entity = spawn_world_tile(&mut commands, &assets, key, world_position, &settings);
            loaded.entities.insert(key, entity);
        }
    }

    let stale: Vec<(TileKey, Entity)> = loaded
        .entities
        .iter()
        .filter_map(|(key, entity)| (!desired.contains(key)).then_some((*key, *entity)))
        .collect();

    for (key, entity) in stale {
        commands.entity(entity).despawn();
        loaded.entities.remove(&key);
    }
}

fn spawn_world_tile(
    commands: &mut Commands,
    assets: &WorldTileAssets,
    key: TileKey,
    world_position: Vec2,
    settings: &GameSettings,
) -> Entity {
    let index = arena_floor_index(key.x, key.y, &settings.world.arena_floor_tiles);
    let mut sprite = Sprite::from_atlas_image(
        assets.arena_image.clone(),
        TextureAtlas {
            layout: assets.arena_layout.clone(),
            index,
        },
    );
    sprite.color = arena_floor_tint(world_position, settings);
    commands
        .spawn((
            WorldTile,
            sprite,
            Transform {
                translation: world_position.extend(0.0),
                ..default()
            },
            Name::new("Arena Floor Tile"),
        ))
        .id()
}

fn arena_floor_tint(world_position: Vec2, settings: &GameSettings) -> Color {
    let distance = world_position.length();
    let tier_index = settings.spawner.tier_index_for_distance(distance) as usize;
    let band = &settings.arena_floor.bands[tier_index];
    let inner_radius = if tier_index == 0 {
        0.0
    } else {
        settings.spawner.tiers[tier_index - 1].max_distance_from_center
    };
    let outer_radius = settings.spawner.tiers[tier_index].max_distance_from_center;
    let span = (outer_radius - inner_radius).max(1.0);
    let t = ((distance - inner_radius) / span).clamp(0.0, 1.0);
    let inner = band.inner_tint.to_linear();
    let outer = band.outer_tint.to_linear();
    Color::LinearRgba(LinearRgba {
        red: inner.red.lerp(outer.red, t),
        green: inner.green.lerp(outer.green, t),
        blue: inner.blue.lerp(outer.blue, t),
        alpha: inner.alpha.lerp(outer.alpha, t),
    })
}

fn arena_floor_index(x: i32, y: i32, arena_floor_tiles: &[usize]) -> usize {
    // Deterministic integer hash: visually varied, stable between frames and
    // clients, and avoids obvious row/column striping.
    let mut value = (x as u32).wrapping_mul(0x9E37_79B9) ^ (y as u32).wrapping_mul(0x85EB_CA6B);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;

    arena_floor_tiles[value as usize % arena_floor_tiles.len()]
}

fn spawn_pit_decorations(
    mut commands: Commands,
    assets: Res<WorldTileAssets>,
    settings: Res<GameSettings>,
) {
    let bounds = settings.world.safezone_bounds();
    let tile_size = settings.world.tile_pixel_size;
    let room_tiles = (bounds.size() / tile_size).as_ivec2();
    let layout_origin = bounds.min;
    let safezone = &settings.world.safezone;
    let columns = safezone.tileset.columns;
    let wall = AtlasTile::from_uvec2(safezone.wall_tile, columns);
    let wall_thickness = safezone.wall_ring_thickness;

    commands.spawn((
        Sprite::from_color(safezone.background_color, bounds.size()),
        Transform::from_translation(bounds.center().extend(PIT_BACKGROUND_Z)),
        Name::new("Pit Background"),
    ));

    for floor in &safezone.floors {
        for y in (floor.y - 1)..(floor.y + floor.height + 1) {
            for x in (floor.x - 1)..(floor.x + floor.width + 1) {
                spawn_pit_tile(
                    &mut commands,
                    &assets,
                    layout_origin,
                    x,
                    y,
                    floor_tile(floor, x, y, columns),
                    tile_size,
                    PIT_FLOOR_Z,
                );
            }
        }
    }

    for y in -wall_thickness..room_tiles.y + wall_thickness {
        for x in -wall_thickness..room_tiles.x + wall_thickness {
            if (0..room_tiles.x).contains(&x) && (0..room_tiles.y).contains(&y) {
                continue;
            }
            spawn_pit_tile(
                &mut commands,
                &assets,
                bounds.min,
                x,
                y,
                wall,
                tile_size,
                PIT_WALL_Z,
            );
        }
    }

    for prop in &safezone.props {
        spawn_pit_tile(
            &mut commands,
            &assets,
            layout_origin,
            prop.x,
            prop.y,
            AtlasTile::from_uvec2(prop.tile, columns),
            tile_size,
            PIT_PROP_Z,
        );
    }
}

fn spawn_pit_tile(
    commands: &mut Commands,
    assets: &WorldTileAssets,
    origin: Vec2,
    x: i32,
    y: i32,
    tile: AtlasTile,
    tile_size: f32,
    z: f32,
) {
    commands.spawn((
        Sprite::from_atlas_image(
            assets.safezone_image.clone(),
            TextureAtlas {
                layout: assets.safezone_layout.clone(),
                index: tile.atlas_index(),
            },
        ),
        Transform {
            translation: (origin + Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * tile_size).extend(z),
            ..default()
        },
    ));
}

fn floor_tile(floor: &SafezoneFloorRect, x: i32, y: i32, columns: u32) -> AtlasTile {
    AtlasTile::from_uvec2(floor.center_tile, columns).offset(
        if x == floor.x - 1 {
            -1
        } else if x == floor.x + floor.width {
            1
        } else {
            0
        },
        if y == floor.y + floor.height {
            -1
        } else if y == floor.y - 1 {
            1
        } else {
            0
        },
    )
}

#[cfg(feature = "dev")]
fn draw_world_boundaries(settings: Res<GameSettings>, mut gizmos: Gizmos) {
    for bounds in [
        settings.world.arena_bounds(),
        settings.world.safezone_bounds(),
    ] {
        gizmos.rect_2d(
            Isometry2d::from_translation(bounds.center()),
            bounds.size(),
            Color::srgb(0.95, 0.25, 0.2),
        );
    }
}
