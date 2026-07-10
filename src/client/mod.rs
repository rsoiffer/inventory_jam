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
use systems::{handle_predicted_spawn, player_movement, receive_message1};

pub struct ExampleClientPlugin;

impl Plugin for ExampleClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, player_movement);

        app.add_systems(Update, receive_message1);
        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_controlled_spawn);
        app.add_observer(handle_interpolated_spawn);
    }
}
