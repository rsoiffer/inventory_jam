//! The client plugin.
//! The client will be responsible for:
//! - connecting to the server at Startup
//! - sending inputs to the server
//! - applying inputs to the locally predicted player (for prediction to work, inputs have to be applied to both the
//!   predicted entity and the server entity)

mod systems;
mod input;

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use input::*;
use systems::*;

use crate::protocol::PlayerMovementInput;
use lightyear::prelude::client::input::InputSystems;


pub struct ExampleClientPlugin;

impl Plugin for ExampleClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, configure_input_delay);

        app.add_systems(
            FixedPreUpdate,
            buffer_input.in_set(InputSystems::WriteClientInputs),
        );

        app.add_systems(FixedUpdate, player_movement);

        app.add_systems(Update, receive_message1);
        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_controlled_spawn);
        app.add_observer(handle_interpolated_spawn);
    }
}