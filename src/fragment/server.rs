use crate::app::ServerState;
use crate::fragment::api::SpawnFragmentPool;
use crate::fragment::{
    protocol::Fragment,
    shared::{
        self as fragment_shared, FragmentLifetime, FragmentMagnetAge, FragmentPosition,
        ServerFragment,
    },
};
use crate::persistence::{CachedPersistentState, PersistenceReady, Transaction};
use crate::player::{PlayerId, PlayerPosition, PlayerUsername};
#[cfg(feature = "dev")]
use crate::protocol::inputs::PlayerAction;
use crate::protocol::rooms::GameRoom;
use crate::settings::GameSettings;
use crate::shared::FixedGameplaySet;
use bevy::prelude::*;
#[cfg(feature = "dev")]
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;
use lightyear::{core::tick::TickDuration, prediction::Predicted};
use rand::Rng;

#[cfg(feature = "dev")]
const MIN_FRAGMENTS_PER_DEBUG_POOL: u32 = 50;
#[cfg(feature = "dev")]
const MAX_FRAGMENTS_PER_DEBUG_POOL: u32 = 150;

/// Installs only server-authoritative fragment behavior.
pub struct FragmentServerPlugin;

impl Plugin for FragmentServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnFragmentPool>();

        #[cfg(feature = "dev")]
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

        #[cfg(not(feature = "dev"))]
        app.add_systems(
            FixedUpdate,
            (spawn_requested_fragment_pools, simulate_server_fragments)
                .chain()
                .before(FixedGameplaySet::Persistence)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

/// Temporary debug adapter: Shift requests a pool from the Fragment feature.
/// Replace this system later with requests emitted by enemy deaths.
#[cfg(feature = "dev")]
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
        let total_value = request.total_value;
        let drop_entity_count = settings.fragment.drop_entity_count;
        let values = if total_value == 0 || drop_entity_count == 0 {
            Vec::new()
        } else {
            let entity_count = (drop_entity_count as u32).min(total_value);
            let base = total_value / entity_count;
            let remainder = total_value % entity_count;
            (0..entity_count)
                .map(|index| base + u32::from(index < remainder))
                .collect()
        };

        info!(
            center = ?request.center,
            total_value,
            entity_count = values.len(),
            "SERVER spawning Fragment pool"
        );

        for value in values {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let radius = rng
                .random_range(
                    settings.fragment.pool_min_radius * settings.fragment.pool_min_radius
                        ..=settings.fragment.pool_max_radius * settings.fragment.pool_max_radius,
                )
                .sqrt();
            let spawn_position = request.center + Vec2::new(angle.cos(), angle.sin()) * radius;

            commands.spawn((
                Fragment::available(spawn_position, value),
                FragmentPosition(spawn_position),
                FragmentLifetime {
                    remaining_ticks: settings.fragment.lifetime_ticks,
                },
                request.room,
                ServerFragment,
                Replicate::to_clients(NetworkTarget::All),
                Name::new("Server Fragment"),
            ));
        }
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
            Option<&FragmentMagnetAge>,
            &GameRoom,
        ),
        With<ServerFragment>,
    >,
    players: Query<(Entity, &PlayerId, &PlayerPosition, Has<Predicted>)>,
    ready_players: Query<(&PlayerUsername, &CachedPersistentState), With<PersistenceReady>>,
    mut transactions: MessageWriter<Transaction>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let is_host_server = !host_server.is_empty();
    let tick_secs = tick_duration.0.as_secs_f32();

    for (fragment_entity, mut fragment, mut position, mut lifetime, magnet_age, room) in
        &mut fragments
    {
        if let Some(collector) = fragment.collector {
            let mut age = magnet_age.copied().unwrap_or_default();
            let despawned = simulate_collected_fragment(
                &mut commands,
                fragment_entity,
                collector,
                &mut position,
                &mut age,
                &players,
                is_host_server,
                tick_secs,
                &settings,
            );
            if !despawned {
                commands.entity(fragment_entity).insert(age);
            }
            continue;
        }

        if let Some((player_entity, collector)) =
            nearest_player_in_collection_range(position.0, &players, is_host_server, &settings)
        {
            let Ok((username, cache)) = ready_players.get(player_entity) else {
                continue;
            };

            transactions.write(Transaction::add_weapon_fragments(
                username.0.clone(),
                cache.equipped_weapon_id,
                fragment.value,
            ));
            fragment.collector = Some(collector);
            commands
                .entity(fragment_entity)
                .insert(FragmentMagnetAge::default());

            info!(
                ?fragment_entity,
                ?player_entity,
                value = fragment.value,
                weapon_id = cache.equipped_weapon_id,
                carried_by = ?collector,
                "SERVER awarded Fragment"
            );

            continue;
        }

        lifetime.remaining_ticks = lifetime.remaining_ticks.saturating_sub(1);

        if lifetime.remaining_ticks == 0 || !room.bounds(&settings.world).contains(position.0) {
            commands.entity(fragment_entity).despawn();
        }
    }
}

fn simulate_collected_fragment(
    commands: &mut Commands,
    fragment_entity: Entity,
    collector: PeerId,
    position: &mut FragmentPosition,
    magnet_age: &mut FragmentMagnetAge,
    players: &Query<(Entity, &PlayerId, &PlayerPosition, Has<Predicted>)>,
    is_host_server: bool,
    tick_secs: f32,
    settings: &GameSettings,
) -> bool {
    let Some(target) = player_position_by_id(collector, players, is_host_server) else {
        return false;
    };

    fragment_shared::pull_fragment_toward(
        position,
        target,
        magnet_age.0,
        tick_secs,
        &settings.fragment,
    );
    magnet_age.0 = magnet_age.0.saturating_add(1);

    if position.0.distance_squared(target) <= settings.fragment.radius * settings.fragment.radius {
        commands.entity(fragment_entity).despawn();
        true
    } else {
        false
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
