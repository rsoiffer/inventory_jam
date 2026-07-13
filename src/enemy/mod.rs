mod protocol;
#[cfg(feature = "gui")]
mod render;
#[cfg(feature = "server")]
mod server;
pub mod shared;

use crate::app::{AppState, game_is_active};
use crate::enemy::render::draw_enemy_boxes;
use crate::enemy::server::spawn_enemy;
use bevy::app::{App, FixedUpdate, Plugin, Update};
use bevy::prelude::{IntoScheduleConfigs, in_state};

pub use protocol::{EnemyHealth, EnemyPosition, register};

pub struct EnemyClientPlugin;

impl Plugin for EnemyClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_enemy_boxes.run_if(game_is_active));
    }
}

pub struct EnemyServerPlugin;

impl Plugin for EnemyServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, spawn_enemy.run_if(in_state(AppState::Hosting)));
    }
}
