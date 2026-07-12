/*
    Allows the protocol to register events.
*/

use bevy::prelude::*;
use lightyear::prelude::*;

pub struct ServerEventsChannel;
pub struct ClientEventsChannel;

pub fn register(app: &mut App) {
    app.add_channel::<ServerEventsChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
        ..default()
    })
    .add_direction(NetworkDirection::ServerToClient);

    app.add_channel::<ClientEventsChannel>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
        ..default()
    })
    .add_direction(NetworkDirection::ClientToServer);
}
