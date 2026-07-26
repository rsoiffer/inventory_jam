use crate::app::{AppState, LocalUsername};
use bevy::prelude::*;

pub static ARENA_WORLD_BOUNDS: Rect =
    Rect::from_center_size(Vec2::new(0.0, 0.0), Vec2::new(1600.0, 1200.0));
pub static SHOP_WORLD_BOUNDS: Rect =
    Rect::from_center_size(Vec2::new(1700.0, 0.0), Vec2::new(1600.0, 1200.0));

pub const GAME_NAME: &str = "Arena of Chapions";

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixedGameplaySet {
    Projectile,
}

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(FixedUpdate, FixedGameplaySet::Projectile);
        app.add_systems(FixedUpdate, update_window_title);
    }
}

fn update_window_title(
    mut window_query: Query<&mut Window>,
    state: Res<State<AppState>>,
    username: Res<LocalUsername>,
) {
    for mut window in window_query.iter_mut() {
        match state.get() {
            AppState::MainMenu | AppState::Connecting => (),
            AppState::Playing => window.title = format!("{}: {}", GAME_NAME, username.0),
            AppState::Hosting => window.title = format!("{}: Server", GAME_NAME),
        }
    }
}
