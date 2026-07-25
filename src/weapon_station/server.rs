use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    app::ServerState,
    persistence::{CachedPersistentState, PersistenceReady, Transaction},
    player::{PlayerPosition, PlayerUsername},
    protocol::rooms::{GameRoom, GameRooms},
    settings::GameSettings,
    shared::WeaponSystemSet,
    weapon::protocol::WeaponCooldown,
    weapon_station::{StationPosition, UpgradeStationKind, WeaponStationId},
};

#[derive(Component)]
struct UpgradeStationArmed(bool);

pub struct WeaponStationServerPlugin;

impl Plugin for WeaponStationServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ServerState::Hosting), spawn_weapon_stations);
        app.add_systems(
            FixedUpdate,
            (
                ensure_upgrade_station_arming,
                apply_weapon_station_overlaps,
                apply_upgrade_station_overlaps,
            )
                .chain()
                .in_set(WeaponSystemSet::Stations)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

fn spawn_weapon_stations(
    mut commands: Commands,
    settings: Res<GameSettings>,
    existing_weapons: Query<(), With<WeaponStationId>>,
    existing_upgrades: Query<(), With<UpgradeStationKind>>,
) {
    let safezone_center = settings.world.safezone_bounds().center();

    if existing_weapons.is_empty() {
        for station in &settings.weapon_stations.stations {
            commands.spawn((
                Name::new(format!("Weapon Station {}", station.weapon_id)),
                WeaponStationId(station.weapon_id),
                StationPosition(safezone_center + station.offset),
                Replicate::to_clients(NetworkTarget::All),
            ));
        }
    }

    if existing_upgrades.is_empty() {
        for station in &settings.weapon_stations.upgrade_stations {
            commands.spawn((
                Name::new(format!("Upgrade Station {:?}", station.kind)),
                UpgradeStationKind(station.kind),
                StationPosition(safezone_center + station.offset),
                Replicate::to_clients(NetworkTarget::All),
            ));
        }
    }
}

fn ensure_upgrade_station_arming(
    mut commands: Commands,
    players: Query<
        Entity,
        (
            With<ControlledBy>,
            With<PlayerPosition>,
            Without<UpgradeStationArmed>,
        ),
    >,
) {
    for entity in &players {
        commands.entity(entity).insert(UpgradeStationArmed(true));
    }
}

fn apply_weapon_station_overlaps(
    settings: Res<GameSettings>,
    mut players: Query<
        (
            &PlayerPosition,
            &GameRoom,
            &PlayerUsername,
            &mut WeaponCooldown,
            &CachedPersistentState,
        ),
        (With<ControlledBy>, With<PersistenceReady>),
    >,
    stations: Query<(&WeaponStationId, &StationPosition)>,
    mut transactions: MessageWriter<Transaction>,
) {
    let trigger_radius_squared = station_trigger_radius_squared(&settings);

    for (position, room, username, mut cooldown, cache) in &mut players {
        if room.room != GameRooms::Safezone {
            continue;
        }

        let Some(station_id) = stations.iter().find_map(|(id, station_position)| {
            (position.0.distance_squared(station_position.0) <= trigger_radius_squared)
                .then_some(id.0)
        }) else {
            continue;
        };

        if cache.equipped_weapon_id == station_id {
            continue;
        }

        *cooldown = WeaponCooldown::default();
        transactions.write(Transaction::equip_weapon(username.0.clone(), station_id));
    }
}

fn apply_upgrade_station_overlaps(
    settings: Res<GameSettings>,
    mut players: Query<
        (
            &PlayerPosition,
            &GameRoom,
            &PlayerUsername,
            &mut UpgradeStationArmed,
            &CachedPersistentState,
        ),
        (With<ControlledBy>, With<PersistenceReady>),
    >,
    stations: Query<(&UpgradeStationKind, &StationPosition)>,
    mut transactions: MessageWriter<Transaction>,
) {
    let trigger_radius_squared = station_trigger_radius_squared(&settings);

    for (position, room, username, mut armed, cache) in &mut players {
        if room.room != GameRooms::Safezone {
            continue;
        }

        let Some(kind) = stations.iter().find_map(|(kind, station_position)| {
            (position.0.distance_squared(station_position.0) <= trigger_radius_squared)
                .then_some(kind.0)
        }) else {
            armed.0 = true;
            continue;
        };

        if !armed.0 {
            continue;
        }
        armed.0 = false;

        let progress = cache.weapon(cache.equipped_weapon_id);
        let level = progress.level_for(kind);
        let Ok(cost) = settings.progression.upgrade_cost(level) else {
            continue;
        };
        if progress.fragments < cost {
            continue;
        }

        transactions.write(Transaction::upgrade_weapon_stat(
            username.0.clone(),
            cache.equipped_weapon_id,
            kind,
            cost,
        ));
    }
}

fn station_trigger_radius_squared(settings: &GameSettings) -> f32 {
    let radius = settings.weapon_stations.trigger_radius + settings.player.collision_radius;
    radius * radius
}
