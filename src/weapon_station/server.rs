use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    app::ServerState,
    persistence::{CachedPersistentState, PersistenceReady, Transaction},
    player::{PlayerPosition, PlayerUsername},
    protocol::rooms::{GameRoom, GameRooms},
    settings::GameSettings,
    shared::{FixedGameplaySet, WeaponSystemSet},
    weapon::protocol::{EquippedWeapon, WeaponCooldown},
    weapon_station::{
        UpgradeStation, UpgradeStationPosition, WeaponStationId, WeaponStationPosition,
    },
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
                .in_set(FixedGameplaySet::Weapon)
                .run_if(in_state(ServerState::Hosting)),
        );
        app.add_systems(
            FixedUpdate,
            sync_equipped_weapon_levels
                .after(FixedGameplaySet::Persistence)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

fn spawn_weapon_stations(
    mut commands: Commands,
    settings: Res<GameSettings>,
    existing_weapons: Query<(), With<WeaponStationId>>,
    existing_upgrades: Query<(), With<UpgradeStation>>,
) {
    let safezone_center = settings.world.safezone_bounds().center();

    if existing_weapons.is_empty() {
        for station in &settings.weapon_stations.stations {
            commands.spawn((
                Name::new(format!("Weapon Station {}", station.weapon_id)),
                WeaponStationId(station.weapon_id),
                WeaponStationPosition(safezone_center + station.offset),
                Replicate::to_clients(NetworkTarget::All),
            ));
        }
    }

    if existing_upgrades.is_empty() {
        commands.spawn((
            Name::new("Upgrade Station"),
            UpgradeStation,
            UpgradeStationPosition(
                safezone_center + settings.weapon_stations.upgrade_station_offset,
            ),
            Replicate::to_clients(NetworkTarget::All),
        ));
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
            &mut EquippedWeapon,
            &mut WeaponCooldown,
            Option<&CachedPersistentState>,
        ),
        With<ControlledBy>,
    >,
    stations: Query<(&WeaponStationId, &WeaponStationPosition)>,
) {
    let trigger_radius = settings.weapon_stations.trigger_radius + settings.player.collision_radius;
    let trigger_radius_squared = trigger_radius * trigger_radius;

    for (position, room, mut equipped, mut cooldown, cache) in &mut players {
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

        let level = cache
            .map(|state| state.weapon(station_id).level)
            .unwrap_or(0);
        *equipped = EquippedWeapon::new(station_id, level);
        *cooldown = WeaponCooldown::default();
    }
}

fn apply_upgrade_station_overlaps(
    settings: Res<GameSettings>,
    mut players: Query<
        (
            &PlayerPosition,
            &GameRoom,
            &PlayerUsername,
            &EquippedWeapon,
            &mut UpgradeStationArmed,
            &CachedPersistentState,
        ),
        (With<ControlledBy>, With<PersistenceReady>),
    >,
    stations: Query<&UpgradeStationPosition, With<UpgradeStation>>,
    mut transactions: MessageWriter<Transaction>,
) {
    let Some(station_position) = stations.iter().next() else {
        return;
    };

    let trigger_radius = settings.weapon_stations.trigger_radius + settings.player.collision_radius;
    let trigger_radius_squared = trigger_radius * trigger_radius;

    for (position, room, username, equipped, mut armed, cache) in &mut players {
        if room.room != GameRooms::Safezone {
            continue;
        }

        let overlapping = position.0.distance_squared(station_position.0) <= trigger_radius_squared;
        if !overlapping {
            armed.0 = true;
            continue;
        }

        if !armed.0 {
            continue;
        }
        armed.0 = false;

        let progress = cache.weapon(equipped.id);
        let cost = match settings.progression.upgrade_cost(progress.level) {
            Ok(cost) => cost,
            Err(error) => {
                warn!(%error, username = %username.0, "rejected weapon upgrade");
                continue;
            }
        };
        if progress.fragments < cost {
            continue;
        }

        transactions.write(Transaction::upgrade_weapon(
            username.0.clone(),
            equipped.id,
            cost,
        ));
    }
}

fn sync_equipped_weapon_levels(
    mut players: Query<
        (&mut EquippedWeapon, &CachedPersistentState),
        (With<ControlledBy>, With<PersistenceReady>),
    >,
) {
    for (mut equipped, cache) in &mut players {
        let level = cache.weapon(equipped.id).level;
        if equipped.level != level {
            equipped.level = level;
        }
    }
}
