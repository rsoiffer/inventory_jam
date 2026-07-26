use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;

use crate::networking::SEND_INTERVAL;

mod interest;

pub struct ExampleServerPlugin;

impl Plugin for ExampleServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReplicationMetadata::new(SEND_INTERVAL));
        app.add_plugins(interest::InterestPlugin);

        app.add_observer(handle_new_client);
    }
}

pub(crate) fn handle_new_client(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert((ReplicationSender, Name::from("Client")));
}
