pub mod channels;
pub mod inputs;
pub mod messages;
pub mod player;
pub mod rooms;

use bevy::prelude::*;

#[derive(Clone)]
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        channels::register(app);
        messages::register(app);
        inputs::register(app);
        player::register(app);
        crate::enemy::register(app);
    }
}
