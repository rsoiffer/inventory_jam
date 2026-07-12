//! The client plugin.
//! The client will be responsible for:
//! - connecting to the server at Startup
//! - sending inputs to the server
//! - applying inputs to the locally predicted player (for prediction to work, inputs have to be applied to both the
//!   predicted entity and the server entity)
pub mod player;
mod systems;

use bevy::prelude::*;

use player::*;
use systems::{
    handle_predicted_spawn, player_movement, receive_message1, sample_cursor_aim, update_camera,
    write_cursor_aim_to_leafwing,
};

use crate::projectile::client::{initialize_projectile, simulate_client_projectiles};
use crate::projectile::shared::projectile_collision_system;

pub struct ExampleClientPlugin;

impl Plugin for ExampleClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_camera, sample_cursor_aim).chain());

        app.add_systems(FixedPreUpdate, write_cursor_aim_to_leafwing);

        app.add_systems(
            FixedUpdate,
            (
                player_movement,
                update_predicted_player_aim_direction,
                simulate_client_projectiles,
                projectile_collision_system,
            )
                .chain(),
        );

        app.add_systems(
            Update,
            (smooth_local_aim_visual, draw_local_aimstick).chain(),
        );

        app.add_systems(Update, receive_message1);

        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_controlled_spawn);
        app.add_observer(handle_interpolated_spawn);
        app.add_observer(initialize_projectile);
    }
}
