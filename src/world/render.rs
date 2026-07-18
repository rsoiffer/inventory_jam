use crate::app::game_is_active;
use crate::shared::{ARENA_WORLD_BOUNDS, SHOP_WORLD_BOUNDS, TILE_SIZE};
use bevy::app::{App, Plugin, Startup, Update};
use bevy::color::Color;
use bevy::image::{ImageLoaderSettings, ImageSampler};
use bevy::math::{Isometry2d, UVec2, Vec2, Vec3};
use bevy::prelude::{
    AssetServer, Assets, Commands, Component, Entity, Gizmos, Handle, Image, IntoScheduleConfigs,
    Res, ResMut, Resource, Sprite, TextureAtlas, TextureAtlasLayout, Transform, Visibility,
};

const ATLAS_COLUMNS: u32 = 9;
const ATLAS_ROWS: u32 = 9;
const BACKGROUND_Z: f32 = -2.0;
const FLOOR_Z: f32 = -1.0;
const WALL_Z: f32 = 0.0;
const PROP_Z: f32 = 1.0;
const WALL_RING_THICKNESS: i32 = 3;

const BLUE_FLOOR: Tile = Tile { x: 1, y: 1 };
const GREY_FLOOR: Tile = Tile { x: 1, y: 4 };
const BLUE_FLOOR_02: Tile = Tile { x: 4, y: 1 };
const GREY_FLOOR_02: Tile = Tile { x: 4, y: 4 };
const WALL: Tile = Tile { x: 6, y: 0 };
const PROP_STOOL: Tile = Tile { x: 7, y: 0 };
const PROP_TABLE: Tile = Tile { x: 8, y: 0 };
const PROP_ROCK: Tile = Tile { x: 6, y: 1 };
const PROP_ANVIL: Tile = Tile { x: 7, y: 1 };
const PROP_BARREL: Tile = Tile { x: 8, y: 1 };

const PIT_BACKGROUNDS: &[ColorRect] = &[ColorRect {
    color: Color::srgb(0.08, 0.1, 0.14),
    x: 0,
    y: 0,
    width: 16,
    height: 12,
}];

const PIT_FLOORS: &[FloorRect] = &[
    FloorRect {
        center: BLUE_FLOOR,
        x: 2,
        y: 2,
        width: 12,
        height: 8,
    },
    FloorRect {
        center: GREY_FLOOR,
        x: 6,
        y: 5,
        width: 4,
        height: 2,
    },
    FloorRect {
        center: BLUE_FLOOR_02,
        x: 3,
        y: 7,
        width: 4,
        height: 3,
    },
    FloorRect {
        center: GREY_FLOOR_02,
        x: 10,
        y: 3,
        width: 3,
        height: 3,
    },
];

const PIT_PROPS: &[PropPlacement] = &[
    PropPlacement {
        tile: PROP_TABLE,
        x: 8,
        y: 6,
    },
    PropPlacement {
        tile: PROP_STOOL,
        x: 7,
        y: 5,
    },
    PropPlacement {
        tile: PROP_ANVIL,
        x: 9,
        y: 5,
    },
    PropPlacement {
        tile: PROP_ROCK,
        x: 10,
        y: 4,
    },
    PropPlacement {
        tile: PROP_BARREL,
        x: 6,
        y: 6,
    },
];

pub struct WorldRenderPlugin;

impl Plugin for WorldRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_pit_tileset, spawn_pit_decorations).chain())
            .add_systems(Update, draw_arena_test_world.run_if(game_is_active));
    }
}

#[derive(Resource)]
struct PitTilesetAssets {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

#[derive(Component)]
struct PitDecorationRoot;

#[derive(Clone, Copy)]
struct Tile {
    x: u32,
    y: u32,
}

impl Tile {
    fn atlas_index(self) -> usize {
        self.y as usize * ATLAS_COLUMNS as usize + self.x as usize
    }

    fn offset(self, dx: i32, dy: i32) -> Self {
        Self {
            x: (self.x as i32 + dx) as u32,
            y: (self.y as i32 + dy) as u32,
        }
    }
}

#[derive(Clone, Copy)]
struct ColorRect {
    color: Color,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Copy)]
struct FloorRect {
    /// Center tile in the tileset; edges/corners are the surrounding 3x3 cells.
    /// `width`/`height` are the interior filled with center tiles; the border
    /// sits one tile outside that rectangle.
    center: Tile,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Copy)]
struct PropPlacement {
    tile: Tile,
    x: i32,
    y: i32,
}

fn load_pit_tileset(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(TILE_SIZE as u32),
        ATLAS_COLUMNS,
        ATLAS_ROWS,
        None,
        None,
    ));

    commands.insert_resource(PitTilesetAssets {
        image: asset_server
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.sampler = ImageSampler::nearest();
            })
            .load("textures/spr_tileset_safezone.png"),
        layout,
    });
}

fn spawn_pit_decorations(mut commands: Commands, assets: Res<PitTilesetAssets>) {
    let origin = SHOP_WORLD_BOUNDS.min;
    let room_tiles = (SHOP_WORLD_BOUNDS.size() / TILE_SIZE).as_ivec2();
    let mut children = Vec::new();

    for background in PIT_BACKGROUNDS {
        children.push(spawn_color_rect(
            &mut commands,
            origin,
            background,
            BACKGROUND_Z,
        ));
    }

    for floor in PIT_FLOORS {
        for gy in (floor.y - 1)..(floor.y + floor.height + 1) {
            for gx in (floor.x - 1)..(floor.x + floor.width + 1) {
                children.push(spawn_tile(
                    &mut commands,
                    &assets,
                    origin,
                    gx,
                    gy,
                    floor_tile(floor, gx, gy),
                    FLOOR_Z,
                ));
            }
        }
    }

    let min = -WALL_RING_THICKNESS;
    let max_x = room_tiles.x + WALL_RING_THICKNESS;
    let max_y = room_tiles.y + WALL_RING_THICKNESS;
    for y in min..max_y {
        for x in min..max_x {
            let inside = (0..room_tiles.x).contains(&x) && (0..room_tiles.y).contains(&y);
            if inside {
                continue;
            }
            children.push(spawn_tile(
                &mut commands,
                &assets,
                origin,
                x,
                y,
                WALL,
                WALL_Z,
            ));
        }
    }

    for prop in PIT_PROPS {
        children.push(spawn_tile(
            &mut commands,
            &assets,
            origin,
            prop.x,
            prop.y,
            prop.tile,
            PROP_Z,
        ));
    }

    commands
        .spawn((
            PitDecorationRoot,
            Transform::default(),
            Visibility::default(),
        ))
        .add_children(&children);
}

fn spawn_color_rect(commands: &mut Commands, origin: Vec2, rect: &ColorRect, z: f32) -> Entity {
    let size = Vec2::new(rect.width as f32, rect.height as f32) * TILE_SIZE;
    let position = origin
        + Vec2::new(
            rect.x as f32 + rect.width as f32 * 0.5,
            rect.y as f32 + rect.height as f32 * 0.5,
        ) * TILE_SIZE;

    commands
        .spawn((
            Sprite::from_color(rect.color, size),
            Transform::from_translation(Vec3::new(position.x, position.y, z)),
        ))
        .id()
}

fn spawn_tile(
    commands: &mut Commands,
    assets: &PitTilesetAssets,
    origin: Vec2,
    grid_x: i32,
    grid_y: i32,
    tile: Tile,
    z: f32,
) -> Entity {
    let position = origin + Vec2::new(grid_x as f32 + 0.5, grid_y as f32 + 0.5) * TILE_SIZE;

    commands
        .spawn((
            Sprite::from_atlas_image(
                assets.image.clone(),
                TextureAtlas {
                    layout: assets.layout.clone(),
                    index: tile.atlas_index(),
                },
            ),
            Transform::from_translation(Vec3::new(position.x, position.y, z)),
        ))
        .id()
}

fn floor_tile(floor: &FloorRect, grid_x: i32, grid_y: i32) -> Tile {
    let left = grid_x == floor.x - 1;
    let right = grid_x == floor.x + floor.width;
    let bottom = grid_y == floor.y - 1;
    let top = grid_y == floor.y + floor.height;

    let dx = if left {
        -1
    } else if right {
        1
    } else {
        0
    };
    let dy = if top {
        -1
    } else if bottom {
        1
    } else {
        0
    };

    floor.center.offset(dx, dy)
}

fn draw_arena_test_world(mut gizmos: Gizmos) {
    let bounds = ARENA_WORLD_BOUNDS;
    let grid_color = Color::srgba(0.35, 0.38, 0.42, 0.35);
    let axis_color = Color::srgba(0.7, 0.72, 0.75, 0.7);
    let boundary_color = Color::srgb(0.95, 0.25, 0.2);

    let mut x = bounds.min.x;
    while x <= bounds.max.x {
        let color = if x.abs() < f32::EPSILON {
            axis_color
        } else {
            grid_color
        };
        gizmos.line_2d(
            Vec2::new(x, bounds.min.y),
            Vec2::new(x, bounds.max.y),
            color,
        );
        x += TILE_SIZE;
    }

    let mut y = bounds.min.y;
    while y <= bounds.max.y {
        let color = if y.abs() < f32::EPSILON {
            axis_color
        } else {
            grid_color
        };
        gizmos.line_2d(
            Vec2::new(bounds.min.x, y),
            Vec2::new(bounds.max.x, y),
            color,
        );
        y += TILE_SIZE;
    }

    gizmos.rect_2d(
        Isometry2d::from_translation(bounds.center()),
        bounds.size(),
        boundary_color,
    );
}
