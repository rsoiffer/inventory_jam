use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    app::ServerState,
    player::protocol::PlayerPosition,
    protocol::rooms::{GameRoom, GameRooms},
    settings::GameSettings,
    shared::{FixedGameplaySet, WeaponSystemSet},
    weapon::protocol::{EquippedWeapon, WeaponCooldown},
    weapon_station::{WeaponStationId, WeaponStationPosition},
};

pub struct WeaponStationServerPlugin;

impl Plugin for WeaponStationServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ServerState::Hosting), spawn_weapon_stations);
        app.add_systems(
            FixedUpdate,
            apply_weapon_station_overlaps
                .in_set(WeaponSystemSet::Stations)
                .in_set(FixedGameplaySet::Weapon)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

fn spawn_weapon_stations(
    mut commands: Commands,
    settings: Res<GameSettings>,
    existing: Query<(), With<WeaponStationId>>,
) {
    if !existing.is_empty() {
        return;
    }

    let safezone_center = settings.world.safezone_bounds().center();
    for station in &settings.weapon_stations.stations {
        commands.spawn((
            Name::new(format!("Weapon Station {}", station.weapon_id)),
            WeaponStationId(station.weapon_id),
            WeaponStationPosition(safezone_center + station.offset),
            Replicate::to_clients(NetworkTarget::All),
        ));
    }
}

fn apply_weapon_station_overlaps(
    settings: Res<GameSettings>,
    mut players: Query<
        (
            &PlayerPosition,
            &GameRoom,
            &mut EquippedWeapon,
            &mut WeaponCooldown,
        ),
        With<ControlledBy>,
    >,
    stations: Query<(&WeaponStationId, &WeaponStationPosition)>,
) {
    let trigger_radius = settings.weapon_stations.trigger_radius + settings.player.collision_radius;
    let trigger_radius_squared = trigger_radius * trigger_radius;

    for (position, room, mut equipped, mut cooldown) in &mut players {
        if room.room != GameRooms::Safezone {
            continue;
        }

        let Some(station_id) = stations.iter().find_map(|(id, station_position)| {
            (position.0.distance_squared(station_position.0) <= trigger_radius_squared)
                .then_some(id.0)
        }) else {
            continue;
        };

        if equipped.id == station_id {
            continue;
        }

        equipped.id = station_id;
        *cooldown = WeaponCooldown::default();
    }
}
