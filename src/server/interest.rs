use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    app::ServerState,
    enemy::{EnemyIdentity, EnemyKind, EnemyPosition, EnemySpawnerPosition},
    fragment::Fragment,
    gate::{GateKind, GatePosition},
    player::{PlayerId, PlayerPosition},
    protocol::rooms::{GameRoom, GameRooms},
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
    enemies: Query<(Entity, &EnemyPosition, &EnemyIdentity, &GameRoom)>,
    spawners: Query<(Entity, &EnemySpawnerPosition, &GameRoom)>,
    fragments: Query<(Entity, &Fragment)>,
    gates: Query<(Entity, &GateKind, &GatePosition)>,
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

        for (entity, position, identity, room) in &enemies {
            let visible = identity.kind == EnemyKind::GrandChampion
                || (room.room == viewer_room.room
                    && viewer_position.0.distance_squared(position.0) <= radius_sq);
            set_visibility(&mut commands, entity, sender, visible);
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

        for (entity, fragment) in &fragments {
            set_visibility(
                &mut commands,
                entity,
                sender,
                viewer_room.room == GameRooms::Arena
                    && viewer_position.0.distance_squared(fragment.position) <= radius_sq,
            );
        }

        for (entity, kind, position) in &gates {
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
