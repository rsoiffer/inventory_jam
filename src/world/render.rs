use crate::app::game_is_active;
use crate::shared::{ARENA_WORLD_BOUNDS, SHOP_WORLD_BOUNDS};
use bevy::app::{App, Plugin, Startup, Update};
use bevy::color::Color;
use bevy::image::{ImageLoaderSettings, ImageSampler};
use bevy::math::{Isometry2d, UVec2, Vec2, Vec3};
use bevy::prelude::{
    AssetServer, Assets, Commands, Component, Entity, Gizmos, Handle, Image, IntoScheduleConfigs,
    Res, ResMut, Resource, Sprite, TextureAtlas, TextureAtlasLayout, Transform, Visibility,
};

const GRID_SPACING: f32 = 100.0;
const TILE_SIZE: f32 = 16.0;
const ATLAS_COLUMNS: u32 = 9;
const ATLAS_ROWS: u32 = 9;
const FLOOR_Z: f32 = -1.0;
const WALL_Z: f32 = 0.0;
const PROP_Z: f32 = 1.0;

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

const PIT_FLOORS: &[FloorRect] = &[
    FloorRect {
        center: BLUE_FLOOR,
        x: 30,
        y: 20,
        width: 40,
        height: 30,
    },
    FloorRect {
        center: GREY_FLOOR,
        x: 42,
        y: 28,
        width: 14,
        height: 12,
    },
    FloorRect {
        center: BLUE_FLOOR_02,
        x: 20,
        y: 45,
        width: 16,
        height: 10,
    },
    FloorRect {
        center: GREY_FLOOR_02,
        x: 60,
        y: 22,
        width: 12,
        height: 10,
    },
];

const PIT_PROPS: &[PropPlacement] = &[
    PropPlacement {
        tile: PROP_TABLE,
        x: 48,
        y: 34,
    },
    PropPlacement {
        tile: PROP_STOOL,
        x: 46,
        y: 32,
    },
    PropPlacement {
        tile: PROP_ANVIL,
        x: 52,
        y: 32,
    },
    PropPlacement {
        tile: PROP_ROCK,
        x: 54,
        y: 30,
    },
    PropPlacement {
        tile: PROP_BARREL,
        x: 44,
        y: 35,
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
struct FloorRect {
    /// Center tile in the tileset; edges/corners are the surrounding 3x3 cells.
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

    for floor in PIT_FLOORS {
        for gy in floor.y..floor.y + floor.height {
            for gx in floor.x..floor.x + floor.width {
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

    for x in 0..room_tiles.x {
        children.push(spawn_tile(
            &mut commands,
            &assets,
            origin,
            x,
            0,
            WALL,
            WALL_Z,
        ));
        children.push(spawn_tile(
            &mut commands,
            &assets,
            origin,
            x,
            room_tiles.y - 1,
            WALL,
            WALL_Z,
        ));
    }
    for y in 1..room_tiles.y - 1 {
        children.push(spawn_tile(
            &mut commands,
            &assets,
            origin,
            0,
            y,
            WALL,
            WALL_Z,
        ));
        children.push(spawn_tile(
            &mut commands,
            &assets,
            origin,
            room_tiles.x - 1,
            y,
            WALL,
            WALL_Z,
        ));
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
    let left = grid_x == floor.x;
    let right = grid_x == floor.x + floor.width - 1;
    let bottom = grid_y == floor.y;
    let top = grid_y == floor.y + floor.height - 1;

    let (dx, dy) = if floor.width == 1 && floor.height == 1 {
        (0, 0)
    } else if left && right {
        (
            0,
            if top {
                -1
            } else if bottom {
                1
            } else {
                0
            },
        )
    } else if top && bottom {
        (
            if left {
                -1
            } else if right {
                1
            } else {
                0
            },
            0,
        )
    } else {
        (
            if left {
                -1
            } else if right {
                1
            } else {
                0
            },
            if top {
                -1
            } else if bottom {
                1
            } else {
                0
            },
        )
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
        x += GRID_SPACING;
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
        y += GRID_SPACING;
    }

    gizmos.rect_2d(
        Isometry2d::from_translation(bounds.center()),
        bounds.size(),
        boundary_color,
    );
}
