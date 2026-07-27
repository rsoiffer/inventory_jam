use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;
use leafwing_input_manager::input_map::InputMap;
use lightyear::input::client::InputSystems;
use lightyear::prelude::{Controlled, Predicted};
use rand::Rng;

use crate::app::ClientState;
use crate::enemy::{EnemyHealth, EnemyPosition};
use crate::gate::{GateKind, GatePosition};
use crate::persistence::CachedPersistentState;
use crate::player::nearest_enemy_aim::aim_and_fire_nearest_enemy;
use crate::player::protocol::{PlayerHealth, PlayerPosition};
use crate::protocol::inputs::PlayerAction;
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::settings::{BotSettings, GameSettings, UpgradeStatKind};
use crate::weapon_station::{StationPosition, UpgradeStationKind};

pub struct BotPlugin;

impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BotController>();
        app.add_systems(
            FixedPreUpdate,
            write_bot_actions_to_leafwing
                .in_set(InputSystems::WriteClientInputs)
                .run_if(in_state(ClientState::Playing)),
        );
    }
}

#[derive(Resource)]
struct BotController {
    move_direction: Vec2,
    re_roll_in: f32,
    upgrade_target: Option<UpgradeStatKind>,
}

impl FromWorld for BotController {
    fn from_world(world: &mut World) -> Self {
        let bot = &world.resource::<GameSettings>().bot;
        Self {
            move_direction: random_unit_direction(),
            re_roll_in: random_re_roll_seconds(bot),
            upgrade_target: None,
        }
    }
}

fn write_bot_actions_to_leafwing(
    time: Res<Time>,
    mut bot: ResMut<BotController>,
    settings: Res<GameSettings>,
    mut players: Query<
        (
            &PlayerPosition,
            &GameRoom,
            &PlayerHealth,
            &CachedPersistentState,
            &mut ActionState<PlayerAction>,
        ),
        (
            With<Controlled>,
            With<Predicted>,
            With<InputMap<PlayerAction>>,
        ),
    >,
    enemies: Query<(&EnemyPosition, &EnemyHealth, &GameRoom)>,
    gates: Query<(&GateKind, &GatePosition)>,
    upgrade_stations: Query<(&UpgradeStationKind, &StationPosition)>,
) {
    let seeking_upgrade = match players.single() {
        Ok((position, room, _, cache, _)) if room.room == GameRooms::Safezone => {
            seek_affordable_upgrade(&mut bot, position.0, cache, &settings, &upgrade_stations)
        }
        Ok(_) => {
            bot.upgrade_target = None;
            false
        }
        Err(_) => false,
    };

    if !seeking_upgrade {
        bot.re_roll_in -= time.delta_secs();
        if bot.re_roll_in <= 0.0 {
            bot.re_roll_in = random_re_roll_seconds(&settings.bot);
            bot.move_direction = match players.single() {
                Ok((position, room, ..)) => {
                    let chance = match room.room {
                        GameRooms::Arena => settings.bot.arena_portal_seek_chance,
                        GameRooms::Safezone => settings.bot.safezone_portal_seek_chance,
                    };
                    let mut nearest = None::<(f32, Vec2)>;
                    if rand::rng().random_bool(chance as f64) {
                        for (kind, gate) in &gates {
                            if kind.source_room() != room.room {
                                continue;
                            }
                            let offset = gate.0 - position.0;
                            let distance_squared = offset.length_squared();
                            if distance_squared > f32::EPSILON
                                && nearest.is_none_or(|(best, _)| distance_squared < best)
                            {
                                nearest = Some((distance_squared, offset));
                            }
                        }
                    }
                    nearest
                        .map(|(_, offset)| offset.normalize_or_zero())
                        .unwrap_or_else(random_unit_direction)
                }
                Err(_) => random_unit_direction(),
            };
        }
    }

    let move_direction = bot.move_direction;
    for (position, room, health, cache, mut actions) in &mut players {
        actions.set_axis_pair(&PlayerAction::Move, move_direction);
        aim_and_fire_nearest_enemy(
            &mut actions,
            position.0,
            room,
            health.current,
            cache,
            &settings,
            &enemies,
        );
    }
}

fn seek_affordable_upgrade(
    bot: &mut BotController,
    player_position: Vec2,
    cache: &CachedPersistentState,
    settings: &GameSettings,
    stations: &Query<(&UpgradeStationKind, &StationPosition)>,
) -> bool {
    let progress = cache.weapon(cache.equipped_weapon_id);
    let trigger_radius = settings.weapon_stations.trigger_radius + settings.player.collision_radius;
    let trigger_radius_squared = trigger_radius * trigger_radius;

    let affordable: Vec<(UpgradeStatKind, Vec2)> = stations
        .iter()
        .filter_map(|(kind, position)| {
            let cost = settings
                .progression
                .upgrade_cost(progress.level_for(kind.0))
                .ok()?;
            (progress.fragments >= cost).then_some((kind.0, position.0))
        })
        .collect();

    if affordable.is_empty() {
        bot.upgrade_target = None;
        return false;
    }

    if bot
        .upgrade_target
        .is_some_and(|target| affordable.iter().all(|(kind, _)| *kind != target))
    {
        bot.upgrade_target = None;
    }

    let on_target = bot.upgrade_target.is_some_and(|target| {
        affordable.iter().any(|(kind, position)| {
            *kind == target && player_position.distance_squared(*position) <= trigger_radius_squared
        })
    });

    if bot.upgrade_target.is_none() || on_target {
        let candidates: Vec<_> = affordable
            .iter()
            .copied()
            .filter(|(_, position)| {
                player_position.distance_squared(*position) > trigger_radius_squared
            })
            .collect();

        if candidates.is_empty() {
            // Step off so the server can re-arm the station overlap.
            let (kind, position) = affordable[0];
            bot.upgrade_target.get_or_insert(kind);
            let away = player_position - position;
            if away.length_squared() > f32::EPSILON {
                bot.move_direction = away.normalize_or_zero();
            }
            return true;
        }

        bot.upgrade_target = Some(candidates[rand::rng().random_range(0..candidates.len())].0);
    }

    let Some(position) = bot.upgrade_target.and_then(|target| {
        affordable
            .iter()
            .find_map(|(kind, position)| (*kind == target).then_some(*position))
    }) else {
        return false;
    };

    let offset = position - player_position;
    if offset.length_squared() > f32::EPSILON {
        bot.move_direction = offset.normalize_or_zero();
    }
    true
}

fn random_unit_direction() -> Vec2 {
    let angle = rand::rng().random_range(0.0..std::f32::consts::TAU);
    Vec2::new(angle.cos(), angle.sin())
}

fn random_re_roll_seconds(bot: &BotSettings) -> f32 {
    rand::rng().random_range(bot.move_re_roll_min_seconds..bot.move_re_roll_max_seconds)
}
