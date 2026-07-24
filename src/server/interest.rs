use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    app::ServerState,
    enemy::{EnemyPosition, EnemySpawnerPosition},
    player::{PlayerId, PlayerPosition},
    portal::{PortalKind, PortalPosition},
    protocol::rooms::GameRoom,
    settings::GameSettings,
    shared::FixedGameplaySet,
};

pub(crate) struct InterestPlugin;

impl Plugin for InterestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            update_interest_visibility
                .after(FixedGameplaySet::Player)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

fn update_interest_visibility(
    mut commands: Commands,
    settings: Res<GameSettings>,
    viewers: Query<(&PlayerPosition, &GameRoom, &ControlledBy), With<PlayerId>>,
    players: Query<(Entity, &PlayerPosition, &GameRoom), With<PlayerId>>,
    enemies: Query<(Entity, &EnemyPosition, &GameRoom)>,
    spawners: Query<(Entity, &EnemySpawnerPosition, &GameRoom)>,
    portals: Query<(Entity, &PortalKind, &PortalPosition)>,
) {
    let radius_sq = settings.network.interest_radius.powi(2);

    for (viewer_position, viewer_room, controlled_by) in &viewers {
        let sender = controlled_by.owner;

        for (entity, position, room) in &players {
            set_visibility(
                &mut commands,
                entity,
                sender,
                room.room == viewer_room.room
                    && viewer_position.0.distance_squared(position.0) <= radius_sq,
            );
        }

        for (entity, position, room) in &enemies {
            set_visibility(
                &mut commands,
                entity,
                sender,
                room.room == viewer_room.room
                    && viewer_position.0.distance_squared(position.0) <= radius_sq,
            );
        }

        for (entity, position, room) in &spawners {
            set_visibility(
                &mut commands,
                entity,
                sender,
                room.room == viewer_room.room
                    && viewer_position.0.distance_squared(position.0) <= radius_sq,
            );
        }

        for (entity, kind, position) in &portals {
            set_visibility(
                &mut commands,
                entity,
                sender,
                kind.source_room() == viewer_room.room
                    && viewer_position.0.distance_squared(position.0) <= radius_sq,
            );
        }
    }
}

fn set_visibility(commands: &mut Commands, entity: Entity, sender: Entity, visible: bool) {
    if visible {
        commands.gain_visibility(entity, sender);
    } else {
        commands.lose_visibility(entity, sender);
    }
}
