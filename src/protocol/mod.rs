//! Replicated protocol definitions shared by clients and servers.

/// Ordered-reliable channel for server-to-client events.
pub mod channels;
/// Leafwing player actions and input map registration.
pub mod inputs;
/// Network messages exchanged between server and clients.
pub mod messages;
/// Replicated player components and spawn bundle.
pub mod player;

use bevy::prelude::*;

// Convenience re-exports for gameplay modules; not every feature set uses every symbol.
#[allow(unused_imports)]
pub use channels::ServerEventsChannel;
#[allow(unused_imports)]
pub use inputs::PlayerAction;
#[allow(unused_imports)]
pub use messages::DebugServerMessage;
#[allow(unused_imports)]
pub use player::{PlayerColor, PlayerId, PlayerPosition};

/// Registers all protocol channels, messages, inputs, and components.
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
