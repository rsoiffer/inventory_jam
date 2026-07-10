pub mod channels;
pub mod inputs;
pub mod messages;
pub mod player;

use bevy::prelude::*;

pub use channels::ServerEventsChannel;
pub use inputs::PlayerAction;
pub use messages::DebugServerMessage;
pub use player::{PlayerColor, PlayerId, PlayerPosition};

#[derive(Clone)]
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        channels::register(app);
        messages::register(app);
        inputs::register(app);
        player::register(app);
    }
}
