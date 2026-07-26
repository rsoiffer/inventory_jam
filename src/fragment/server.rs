use crate::app::ServerState;
use crate::fragment::api::SpawnFragmentPool;
use crate::fragment::{
    protocol::{Fragment, FragmentPhase},
    shared as fragment_shared,
};
use crate::persistence::{CachedPersistentState, PersistenceReady, Transaction};
use crate::player::{PlayerId, PlayerPosition, PlayerUsername};
#[cfg(feature = "dev")]
use crate::protocol::inputs::PlayerAction;
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::settings::GameSettings;
use crate::shared::FixedGameplaySet;
use bevy::prelude::*;
#[cfg(feature = "dev")]
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;
use lightyear::{core::tick::TickDuration, prelude::LocalTimeline};
use rand::Rng;

#[cfg(feature = "dev")]
const MIN_FRAGMENTS_PER_DEBUG_POOL: u32 = 50;
#[cfg(feature = "dev")]
const MAX_FRAGMENTS_PER_DEBUG_POOL: u32 = 150;

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

#[cfg(feature = "dev")]
fn request_debug_fragment_pool(
    players: Query<(&PlayerPosition, &GameRoom, &ActionState<PlayerAction>)>,
    mut requests: MessageWriter<SpawnFragmentPool>,
) {
    let mut rng = rand::rng();

    for (player_position, room, actions) in &players {
        if room.room != GameRooms::Arena || !actions.just_pressed(&PlayerAction::UseSkill) {
            continue;
        }
        requests.write(SpawnFragmentPool::new(
            player_position.0,
            rng.random_range(MIN_FRAGMENTS_PER_DEBUG_POOL..=MAX_FRAGMENTS_PER_DEBUG_POOL),
        ));
    }
}

fn spawn_requested_fragment_pools(
    mut commands: Commands,
    settings: Res<GameSettings>,
    mut requests: MessageReader<SpawnFragmentPool>,
) {
    let mut rng = rand::rng();

    for request in requests.read() {
        if request.total_value == 0 || settings.fragment.drop_entity_count == 0 {
            continue;
        }

        let entity_count = (settings.fragment.drop_entity_count as u32).min(request.total_value);
        let base = request.total_value / entity_count;
        let remainder = request.total_value % entity_count;

        for index in 0..entity_count {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let radius = rng
                .random_range(
                    settings.fragment.pool_min_radius * settings.fragment.pool_min_radius
                        ..=settings.fragment.pool_max_radius * settings.fragment.pool_max_radius,
                )
                .sqrt();
            let position = request.center + Vec2::new(angle.cos(), angle.sin()) * radius;

            commands.spawn((
                Fragment {
                    value: base + u32::from(index < remainder),
                    phase: FragmentPhase::OnGround,
                    position,
                    movement: Vec2::ZERO,
                    remaining_ticks: settings.fragment.lifetime_ticks,
                },
                Replicate::to_clients(NetworkTarget::All),
                Name::new("Fragment"),
            ));
        }
    }
}

fn simulate_server_fragments(
    mut commands: Commands,
    mut fragments: Query<(Entity, &mut Fragment)>,
    players: Query<(Entity, &PlayerId, &PlayerPosition, &GameRoom)>,
    ready_players: Query<(&PlayerUsername, &CachedPersistentState), With<PersistenceReady>>,
    mut transactions: MessageWriter<Transaction>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();
    let collect_r2 = settings.fragment.player_collection_radius.powi(2);
    let arena = settings.world.arena_bounds();

    for (entity, mut fragment) in &mut fragments {
        if matches!(fragment.phase, FragmentPhase::OnGround) {
            fragment.movement = Vec2::ZERO;

            let nearest = players
                .iter()
                .filter(|(_, _, _, room)| room.room == GameRooms::Arena)
                .filter_map(|(player, id, position, _)| {
                    let d2 = position.0.distance_squared(fragment.position);
                    (d2 <= collect_r2).then_some((player, id.0, d2))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2));

            if let Some((player, collector, _)) = nearest {
                if ready_players.get(player).is_err() {
                    continue;
                }
                fragment.phase = FragmentPhase::StartingPull {
                    collector,
                    start_tick: tick,
                };
            } else {
                fragment.remaining_ticks = fragment.remaining_ticks.saturating_sub(1);
                if fragment.remaining_ticks == 0 || !arena.contains(fragment.position) {
                    commands.entity(entity).despawn();
                }
            }
            continue;
        }

        let previous_phase = fragment.phase;
        let target = fragment.phase.collector().and_then(|collector| {
            fragment_shared::collector_position(
                collector,
                players.iter().filter_map(|(_, id, position, room)| {
                    (room.room == GameRooms::Arena).then_some((id.0, position.0, false))
                }),
            )
        });

        if target.is_none() {
            commands.entity(entity).despawn();
            continue;
        }

        fragment_shared::tick_fragment(&mut fragment, target, tick, tick_secs, &settings.fragment);

        if matches!(previous_phase, FragmentPhase::Moving { .. })
            && let FragmentPhase::CollectingImpact { collector, .. } = fragment.phase
            && let Some(player) = players
                .iter()
                .find_map(|(player, id, _, _)| (id.0 == collector).then_some(player))
            && let Ok((username, cache)) = ready_players.get(player)
        {
            transactions.write(Transaction::add_weapon_fragments(
                username.0.clone(),
                cache.equipped_weapon_id,
                fragment.value,
            ));
        }

        if matches!(fragment.phase, FragmentPhase::Gone) {
            commands.entity(entity).despawn();
        }
    }
}
