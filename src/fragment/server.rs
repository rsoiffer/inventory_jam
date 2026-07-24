use crate::app::ServerState;
use crate::fragment::api::SpawnFragmentPool;
use crate::fragment::{
    protocol::Fragment,
    shared::{self as fragment_shared, FragmentLifetime, FragmentPosition, ServerFragment},
};
use crate::persistence::{PersistenceReady, Transaction};
use crate::player::{PlayerId, PlayerPosition, PlayerUsername};
use crate::protocol::inputs::PlayerAction;
use crate::protocol::rooms::GameRoom;
use crate::settings::GameSettings;
use crate::shared::FixedGameplaySet;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;
use rand::Rng;

const MIN_FRAGMENTS_PER_DEBUG_POOL: u16 = 50;
const MAX_FRAGMENTS_PER_DEBUG_POOL: u16 = 150;

/// Installs only server-authoritative fragment behavior.
pub struct FragmentServerPlugin;

impl Plugin for FragmentServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnFragmentPool>();

        app.add_systems(
            FixedUpdate,
            (
                request_debug_fragment_pool,
                spawn_requested_fragment_pools,
                simulate_server_fragments,
            )
                .chain()
                .before(FixedGameplaySet::Persistence)
                .run_if(in_state(ServerState::Hosting)),
        );
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
    settings: Res<GameSettings>,
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
            &settings,
        );
    }
}

fn spawn_fragment_pool(
    commands: &mut Commands,
    rng: &mut impl Rng,
    center: Vec2,
    count: u16,
    room: GameRoom,
    settings: &GameSettings,
) {
    info!(?center, count, "SERVER spawning Fragment pool");

    for _ in 0..count {
        let spawn_position = random_position_in_annulus(
            rng,
            center,
            settings.fragment.pool_min_radius,
            settings.fragment.pool_max_radius,
        );

        commands.spawn((
            Fragment::available(spawn_position),
            FragmentPosition(spawn_position),
            FragmentLifetime {
                remaining_ticks: settings.fragment.lifetime_ticks,
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
    ready_players: Query<&PlayerUsername, With<PersistenceReady>>,
    mut transactions: MessageWriter<Transaction>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
    settings: Res<GameSettings>,
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
                &settings,
            );
            continue;
        }

        if let Some((player_entity, collector)) =
            nearest_player_in_collection_range(position.0, &players, is_host_server, &settings)
        {
            let Ok(username) = ready_players.get(player_entity) else {
                // Player is not persistence-ready yet; leave the fragment available.
                continue;
            };

            transactions.write(Transaction::add_fragments(username.0.clone(), 1));
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

        if lifetime.remaining_ticks == 0 || fragment_is_outside_world(position.0, room, &settings) {
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
    settings: &GameSettings,
) {
    if let Some(target) = player_position_by_id(collector, players, is_host_server) {
        fragment_shared::pull_fragment_toward(position, target, &settings.fragment);
        if position.0.distance_squared(target)
            <= settings.fragment.radius * settings.fragment.radius
        {
            commands.entity(fragment_entity).despawn();
        }
    }
}

fn nearest_player_in_collection_range(
    fragment_position: Vec2,
    players: &Query<(Entity, &PlayerId, &PlayerPosition, Has<Predicted>)>,
    is_host_server: bool,
    settings: &GameSettings,
) -> Option<(Entity, PeerId)> {
    let collection_radius_squared =
        settings.fragment.player_collection_radius * settings.fragment.player_collection_radius;

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

fn fragment_is_outside_world(position: Vec2, room: &GameRoom, settings: &GameSettings) -> bool {
    !room.bounds(&settings.world).contains(position)
}
