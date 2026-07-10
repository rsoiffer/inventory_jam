//! Client gameplay systems for input, prediction, and spawn visuals.

/// Client-side player input binding and spawn observers.
pub mod player;
/// Client movement, messaging, and predicted-entity systems.
mod systems;

use bevy::prelude::*;
use player::*;
use systems::{handle_predicted_spawn, player_movement, receive_debug_server_message};

/// Installs client gameplay systems and spawn observers.
pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, player_movement);

        app.add_systems(Update, receive_debug_server_message);
        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_controlled_spawn);
        app.add_observer(handle_interpolated_spawn);
    }
}
