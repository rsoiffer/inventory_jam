use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;
use rand::Rng;

use crate::protocol::rooms::GameRoom;
use crate::{
    fragment::{
        SpawnFragmentPool,
        protocol::{CarriedFragments, Fragment},
        shared::{self as fragment_shared, FragmentLifetime, FragmentPosition, ServerFragment},
    },
    protocol::{PlayerAction, PlayerId, PlayerPosition},
};

const MIN_FRAGMENTS_PER_DEBUG_POOL: u16 = 50;
const MAX_FRAGMENTS_PER_DEBUG_POOL: u16 = 150;

// The minimum is deliberately larger than the normal collection radius so a
// debug pool does not disappear on the same tick that it spawns.
const FRAGMENT_POOL_MIN_RADIUS: f32 = 50.0;
const FRAGMENT_POOL_MAX_RADIUS: f32 = 200.0;

/// Ensures every authoritative player has fragment currency state owned by this
/// feature. Predicted host-client copies are deliberately ignored.
pub(crate) fn ensure_player_fragment_wallets(
    mut commands: Commands,
    players: Query<(Entity, Has<Predicted>), (With<PlayerId>, Without<CarriedFragments>)>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (entity, predicted) in &players {
        if is_host_server && predicted {
            continue;
        }

        commands.entity(entity).insert(CarriedFragments::default());
    }
}

/// Temporary debug adapter: Shift requests a pool from the Fragment feature.
/// Replace this system later with requests emitted by enemy deaths.
pub(crate) fn request_debug_fragment_pool(
    players: Query<(
        &PlayerPosition,
        &GameRoom,
        &ActionState<PlayerAction>,
        Has<Predicted>,
    )>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
    mut requests: MessageWriter<SpawnFragmentPool>,
) {
    let is_host_server = !host_server.is_empty();
    let mut rng = rand::rng();

    for (player_position, room, actions, predicted) in &players {
        if is_host_server && predicted {
            continue;
        }

        if !actions.just_pressed(&PlayerAction::UseSkill) {
            continue;
        }

        requests.write(SpawnFragmentPool::new(
            player_position.0,
            rng.random_range(MIN_FRAGMENTS_PER_DEBUG_POOL..=MAX_FRAGMENTS_PER_DEBUG_POOL),
            *room,
        ));
    }
}

/// The only system that converts pool requests into authoritative replicated
/// fragment entities.
pub(crate) fn spawn_requested_fragment_pools(
    mut commands: Commands,
    mut requests: MessageReader<SpawnFragmentPool>,
) {
    let mut rng = rand::rng();

    for request in requests.read() {
        spawn_fragment_pool(
            &mut commands,
            &mut rng,
            request.center,
            request.count,
            request.room,
        );
    }
}

fn spawn_fragment_pool(
    commands: &mut Commands,
    rng: &mut impl Rng,
    center: Vec2,
    count: u16,
    room: GameRoom,
) {
    info!(?center, count, "SERVER spawning Fragment pool");

    for _ in 0..count {
        let spawn_position = random_position_in_annulus(
            rng,
            center,
            FRAGMENT_POOL_MIN_RADIUS,
            FRAGMENT_POOL_MAX_RADIUS,
        );

        commands.spawn((
            Fragment::available(spawn_position),
            FragmentPosition(spawn_position),
            FragmentLifetime {
                remaining_ticks: fragment_shared::FRAGMENT_LIFETIME_TICKS,
            },
            room,
            ServerFragment,
            Replicate::to_clients(NetworkTarget::All),
            Name::new("Server Fragment"),
        ));
    }
}

pub(crate) fn simulate_server_fragments(
    mut commands: Commands,
    mut fragments: Query<
        (
            Entity,
            &mut Fragment,
            &mut FragmentPosition,
            &mut FragmentLifetime,
            &GameRoom,
        ),
        With<ServerFragment>,
    >,
    players: Query<(Entity, &PlayerId, &PlayerPosition, Has<Predicted>)>,
    mut wallets: Query<&mut CarriedFragments>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (fragment_entity, mut fragment, mut position, mut lifetime, room) in &mut fragments {
        if let Some(collector) = fragment.collector {
            simulate_collected_fragment(
                &mut commands,
                fragment_entity,
                collector,
                &mut position,
                &players,
                is_host_server,
            );
            continue;
        }

        if let Some((player_entity, collector)) =
            nearest_player_in_collection_range(position.0, &players, is_host_server)
        {
            match wallets.get_mut(player_entity) {
                Ok(mut carried_fragments) => {
                    carried_fragments.0 = carried_fragments.0.saturating_add(1);
                }
                Err(_) => {
                    // This keeps collection correct even if a player was spawned
                    // immediately before this system ran.
                    commands.entity(player_entity).insert(CarriedFragments(1));
                }
            }

            fragment.collector = Some(collector);

            info!(
                ?fragment_entity,
                ?player_entity,
                carried_by = ?collector,
                "SERVER awarded Fragment"
            );

            continue;
        }

        lifetime.remaining_ticks = lifetime.remaining_ticks.saturating_sub(1);

        if lifetime.remaining_ticks == 0 || fragment_is_outside_world(position.0, room) {
            commands.entity(fragment_entity).despawn();
        }
    }
}

fn simulate_collected_fragment(
    commands: &mut Commands,
    fragment_entity: Entity,
    collector: PeerId,
    position: &mut FragmentPosition,
    players: &Query<(Entity, &PlayerId, &PlayerPosition, Has<Predicted>)>,
    is_host_server: bool,
) {
    if let Some(target) = player_position_by_id(collector, players, is_host_server) {
        fragment_shared::pull_fragment_toward(position, target);
        if position.0.distance_squared(target)
            <= fragment_shared::FRAGMENT_RADIUS * fragment_shared::FRAGMENT_RADIUS
        {
            commands.entity(fragment_entity).despawn();
        }
    }
}

fn nearest_player_in_collection_range(
    fragment_position: Vec2,
    players: &Query<(Entity, &PlayerId, &PlayerPosition, Has<Predicted>)>,
    is_host_server: bool,
) -> Option<(Entity, PeerId)> {
    let collection_radius_squared =
        fragment_shared::PLAYER_COLLECTION_RADIUS * fragment_shared::PLAYER_COLLECTION_RADIUS;

    players
        .iter()
        .filter(|(_, _, _, predicted)| !(is_host_server && *predicted))
        .filter_map(|(entity, player_id, player_position, _)| {
            let distance_squared = player_position.0.distance_squared(fragment_position);

            (distance_squared <= collection_radius_squared).then_some((
                entity,
                player_id.0,
                distance_squared,
            ))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(entity, player_id, _)| (entity, player_id))
}

fn player_position_by_id(
    player_id: PeerId,
    players: &Query<(Entity, &PlayerId, &PlayerPosition, Has<Predicted>)>,
    is_host_server: bool,
) -> Option<Vec2> {
    players
        .iter()
        .find(|(_, candidate_id, _, predicted)| {
            candidate_id.0 == player_id && !(is_host_server && *predicted)
        })
        .map(|(_, _, position, _)| position.0)
}

fn random_position_in_annulus(
    rng: &mut impl Rng,
    center: Vec2,
    min_radius: f32,
    max_radius: f32,
) -> Vec2 {
    debug_assert!(min_radius >= 0.0);
    debug_assert!(max_radius >= min_radius);

    let angle = rng.random_range(0.0..std::f32::consts::TAU);
    let radius = rng
        .random_range(min_radius * min_radius..=max_radius * max_radius)
        .sqrt();
    let direction = Vec2::new(angle.cos(), angle.sin());

    center + direction * radius
}

fn fragment_is_outside_world(position: Vec2, room: &GameRoom) -> bool {
    !room.bounds().contains(position)
}
