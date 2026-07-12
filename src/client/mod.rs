//! The client plugin.
//! The client sends inputs, predicts its controlled player, and locally
//! simulates replicated projectile presentation.
pub mod player;
mod systems;

use bevy::prelude::*;

use crate::app::AppState;
use player::*;
use systems::{
    handle_predicted_spawn, player_movement, receive_message1, sample_cursor_aim,
    send_local_username, update_camera, write_cursor_aim_to_leafwing,
};

use crate::projectile::client::{
    initialize_projectile, initialize_projectile_impact, simulate_client_projectiles,
};

pub struct ExampleClientPlugin;

impl Plugin for ExampleClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_camera, sample_cursor_aim)
                .chain()
                .run_if(in_state(AppState::Playing)),
        );

        app.add_systems(
            FixedPreUpdate,
            write_cursor_aim_to_leafwing.run_if(in_state(AppState::Playing)),
        );

        app.add_systems(
            FixedUpdate,
            (
                player_movement,
                update_predicted_player_aim_direction,
                simulate_client_projectiles,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        );

        app.add_systems(
            Update,
            (smooth_local_aim_visual, draw_local_aimstick)
                .chain()
                .run_if(in_state(AppState::Playing)),
        );

        app.add_systems(Update, receive_message1.run_if(in_state(AppState::Playing)));

        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_controlled_spawn);
        app.add_observer(handle_interpolated_spawn);
        app.add_observer(initialize_projectile);
        app.add_observer(initialize_projectile_impact);
        app.add_observer(send_local_username);
    }
}
