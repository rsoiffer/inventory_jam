/*
    Hooks up input to Player when they are spawned.
*/

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

use crate::client::systems::CachedCursorAim;
use crate::protocol::inputs::PlayerAction;
use crate::protocol::player::PlayerId;

pub(crate) fn handle_controlled_spawn(
    trigger: On<Add, Controlled>,
    mut commands: Commands,
    players: Query<(&PlayerId, Option<&ControlledBy>), Without<InputMap<PlayerAction>>>,
    clients: Query<(), With<Client>>,
) {
    let entity = trigger.entity;

    let Ok((player_id, controlled_by)) = players.get(entity) else {
        return;
    };

    if let Some(controlled_by) = controlled_by
        && clients.get(controlled_by.owner).is_err()
    {
        return;
    }

    //info!("Adding Leafwing InputMap to controlled player {entity:?} {player_id:?}");

    commands.entity(entity).insert((
        Name::new(format!("Player (Controlled): {}", player_id.0)),
        PlayerAction::default_input_map(),
        CachedCursorAim::default(),
    ));
}
