//! Ordered-reliable channel used for server-to-client events.

use bevy::prelude::*;
use lightyear::prelude::*;

/// Channel for reliable ordered server event messages.
pub struct ServerEventsChannel;

/// Registers the server events channel on the app.
pub fn register(app: &mut App) {
    app.add_channel::<ServerEventsChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
        ..default()
    })
    .add_direction(NetworkDirection::ServerToClient);
}
