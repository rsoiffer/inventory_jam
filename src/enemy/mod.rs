use crate::app::game_is_active;
use crate::enemy::render::draw_enemy_boxes;
use bevy::app::{App, Plugin, Update};
use bevy::prelude::IntoScheduleConfigs;

pub mod protocol;

#[cfg(feature = "gui")]
mod render;

#[cfg(feature = "server")]
pub(crate) mod server;

pub mod shared;

pub struct EnemyClientPlugin;

impl Plugin for EnemyClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (draw_enemy_boxes,).run_if(game_is_active));
    }
}
