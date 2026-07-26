use crate::app::ServerState;
use crate::combat::HitFlash;
use crate::enemy::{EnemyHealth, EnemyIdentity, EnemyPosition};
use crate::gate::GateTeleportCooldown;
use crate::persistence::{CachedPersistentState, PersistenceReady, Transaction};
use crate::player::PlayerId;
use crate::player::api::AddAristeiaPoints;
use crate::player::protocol::{
    PlayerAimDirection, PlayerAristeia, PlayerBundle, PlayerHealth, PlayerPosition, PlayerUsername,
    PlayerVisual,
};
use crate::player::shared::{
    apply_damage_to_player, apply_player_aim, apply_player_movement, armor_from_cache,
};
use crate::protocol::inputs::PlayerAction;
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::settings::GameSettings;
use crate::shared::FixedGameplaySet;
use bevy::prelude::*;
#[cfg(feature = "gui")]
use bevy::window::PrimaryWindow;
use leafwing_input_manager::prelude::*;
use lightyear::connection::client::Connected;
use lightyear::connection::client_of::ClientOf;
use lightyear::prelude::*;
use rand::Rng;

#[derive(Component, Clone, Copy, Debug, Default)]
struct EnemyContactDamageCooldown {
    remaining_seconds: f32,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct PlayerDeathTimer {
    pub(crate) remaining_seconds: f32,
}

#[derive(Component, Clone, Copy, Debug, Default)]
struct ArenaHealthRegenAccumulator {
    remaining: f32,
}

pub struct PlayerServerPlugin;

impl Plugin for PlayerServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_connected);

        app.add_systems(
            FixedUpdate,
            (
                authoritative_player_movement,
                authoritative_player_aim,
                add_requested_aristeia,
                sync_equipped_max_health,
                restore_safezone_health,
                regenerate_arena_health,
                apply_enemy_contact_damage,
                tick_player_death,
                tick_player_aristeia,
            )
                .chain()
                .in_set(FixedGameplaySet::Player)
                .run_if(in_state(ServerState::Hosting)),
        );

        #[cfg(feature = "gui")]
        app.add_systems(
            FixedUpdate,
            control_dedicated_server_camera
                .in_set(FixedGameplaySet::Player)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

pub(crate) fn add_requested_aristeia(
    settings: Res<GameSettings>,
    mut requests: MessageReader<AddAristeiaPoints>,
    mut players: Query<(&PlayerId, &mut PlayerAristeia)>,
) {
    for AddAristeiaPoints(owner_id, points) in requests.read() {
        let Some(mut aristeia) = players
            .iter_mut()
            .find_map(|(player_id, aristeia)| (player_id.0 == *owner_id).then_some(aristeia))
        else {
            warn!(?owner_id, points, "No player matched the Aristeia award");
            continue;
        };

        aristeia.progress = aristeia.progress.saturating_add(*points);
        loop {
            let cost = settings
                .global_aristeia
                .personal_level_cost(aristeia.current);
            if aristeia.progress < cost {
                break;
            }
            aristeia.progress -= cost;
            aristeia.current = aristeia.current.saturating_add(1);
        }

        let duration = settings
            .global_aristeia
            .personal_duration_seconds(aristeia.current);
        aristeia.maximum_seconds = duration;
        aristeia.remaining_seconds = duration;
    }
}

fn tick_player_aristeia(
    time: Res<Time>,
    settings: Res<GameSettings>,
    mut players: Query<(&GameRoom, &mut PlayerAristeia)>,
) {
    let dt = time.delta_secs();
    for (room, mut aristeia) in &mut players {
        if room.room == GameRooms::Safezone {
            *aristeia = PlayerAristeia::default();
            continue;
        }

        if aristeia.current == 0 && aristeia.progress == 0 {
            aristeia.remaining_seconds = 0.0;
            continue;
        }

        if aristeia.remaining_seconds > 0.0 {
            aristeia.remaining_seconds = (aristeia.remaining_seconds - dt).max(0.0);
            if aristeia.remaining_seconds > 0.0 {
                continue;
            }
        }

        if aristeia.current > 0 {
            aristeia.current -= 1;
            aristeia.progress = 0;
            let duration = settings
                .global_aristeia
                .personal_duration_seconds(aristeia.current);
            aristeia.maximum_seconds = duration;
            aristeia.remaining_seconds = if aristeia.current > 0 { duration } else { 0.0 };
        } else {
            aristeia.progress = 0;
            aristeia.remaining_seconds = 0.0;
        }
    }
}

pub(crate) fn handle_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    settings: Res<GameSettings>,
    mut commands: Commands,
) {
    let Ok(client_id) = query.get(trigger.entity) else {
        return;
    };

    let client_id = client_id.0;
    let spawn_position = settings.world.safezone_bounds().center();

    let entity = commands
        .spawn((
            Name::new(format!("Player: {}", client_id)),
            PlayerBundle::new(
                client_id,
                spawn_position,
                settings.player.maximum_health,
                settings.projectile.buffer_capacity,
            ),
            PlayerVisual,
            EnemyContactDamageCooldown::default(),
            PlayerDeathTimer::default(),
            ArenaHealthRegenAccumulator::default(),
            GateTeleportCooldown {
                remaining_ticks: settings.gate.teleport_cooldown_ticks,
            },
            ActionState::<PlayerAction>::default(),
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::Single(client_id)),
            InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(client_id)),
            ControlledBy {
                owner: trigger.entity,
                lifetime: Default::default(),
            },
        ))
        .id();

    info!(
        ?entity,
        ?client_id,
        ?spawn_position,
        "Created player entity"
    );
}

pub(crate) fn authoritative_player_movement(
    settings: Res<GameSettings>,
    mut players: Query<
        (
            &mut PlayerPosition,
            &GameRoom,
            &PlayerHealth,
            &ActionState<PlayerAction>,
        ),
        With<PlayerId>,
    >,
) {
    for (mut position, room, health, actions) in &mut players {
        apply_player_movement(&mut position, room, health, actions, &settings);
    }
}

pub(crate) fn authoritative_player_aim(
    mut players: Query<(&ActionState<PlayerAction>, &mut PlayerAimDirection), With<PlayerId>>,
) {
    for (actions, mut aim_direction) in &mut players {
        apply_player_aim(&mut aim_direction, actions);
    }
}

fn sync_equipped_max_health(
    settings: Res<GameSettings>,
    mut players: Query<
        (&CachedPersistentState, &mut PlayerHealth),
        (With<ControlledBy>, With<PersistenceReady>),
    >,
) {
    for (cache, mut health) in &mut players {
        let progress = cache.weapon(cache.equipped_weapon_id);
        let maximum = settings
            .progression
            .max_health_at(settings.player.maximum_health, progress.max_health_level);
        if health.maximum == maximum {
            continue;
        }
        let was_full = health.current == health.maximum;
        health.maximum = maximum;
        health.current = if was_full {
            maximum
        } else {
            health.current.min(maximum)
        };
    }
}

fn restore_safezone_health(mut players: Query<(&GameRoom, &mut PlayerHealth), With<ControlledBy>>) {
    for (room, mut health) in &mut players {
        if room.room == GameRooms::Safezone
            && health.current > 0
            && health.current != health.maximum
        {
            health.current = health.maximum;
        }
    }
}

fn regenerate_arena_health(
    time: Res<Time>,
    settings: Res<GameSettings>,
    mut players: Query<
        (
            &GameRoom,
            &mut PlayerHealth,
            &mut ArenaHealthRegenAccumulator,
        ),
        With<ControlledBy>,
    >,
) {
    let regen_rate = settings.player.arena_health_regen_fraction_per_second;
    if regen_rate <= 0.0 {
        return;
    }

    for (room, mut health, mut accumulator) in &mut players {
        if room.room != GameRooms::Arena || health.current == 0 || health.current >= health.maximum
        {
            accumulator.remaining = 0.0;
            continue;
        }

        accumulator.remaining += health.maximum as f32 * regen_rate * time.delta_secs();
        let heal = accumulator.remaining.floor() as u32;
        if heal == 0 {
            continue;
        }
        accumulator.remaining -= heal as f32;
        health.current = (health.current + heal).min(health.maximum);
    }
}

fn apply_enemy_contact_damage(
    time: Res<Time>,
    mut players: Query<
        (
            &PlayerPosition,
            &GameRoom,
            &mut PlayerHealth,
            &mut HitFlash,
            &mut EnemyContactDamageCooldown,
            &mut PlayerDeathTimer,
            Option<&CachedPersistentState>,
            Option<&PlayerUsername>,
            Has<PersistenceReady>,
        ),
        With<ControlledBy>,
    >,
    enemies: Query<(&EnemyPosition, &EnemyHealth, &EnemyIdentity)>,
    settings: Res<GameSettings>,
    mut transactions: MessageWriter<Transaction>,
) {
    let mut rng = rand::rng();
    for (
        player_position,
        room,
        mut health,
        mut flash,
        mut cooldown,
        mut death_timer,
        cache,
        username,
        persistence_ready,
    ) in &mut players
    {
        cooldown.remaining_seconds = (cooldown.remaining_seconds - time.delta_secs()).max(0.0);

        if room.room != GameRooms::Arena || health.current == 0 || cooldown.remaining_seconds > 0.0
        {
            continue;
        }

        let Some(raw_damage) =
            enemies
                .iter()
                .find_map(|(enemy_position, enemy_health, identity)| {
                    let contact_damage = identity.stats(&settings).contact_damage;
                    if enemy_health.current == 0 || contact_damage == 0 {
                        return None;
                    }

                    let collision_radius = settings.player.collision_radius
                        + identity.kind.scale(settings.enemy.collision_radius);
                    (player_position.0.distance_squared(enemy_position.0)
                        <= collision_radius * collision_radius)
                        .then_some(contact_damage)
                })
        else {
            continue;
        };

        let armor = armor_from_cache(&settings, cache);
        if apply_damage_to_player(&mut health, raw_damage, armor, &mut rng) > 0 {
            flash.trigger();
        }
        cooldown.remaining_seconds = settings.player.enemy_contact_damage_interval_seconds;

        if health.current == 0 {
            death_timer.remaining_seconds = settings.player.death_screen_duration_seconds;
            apply_death_penalty(
                &mut transactions,
                &settings,
                cache,
                username,
                persistence_ready,
                &mut rng,
            );
        }
    }
}

pub(crate) fn apply_death_penalty(
    transactions: &mut MessageWriter<Transaction>,
    settings: &GameSettings,
    cache: Option<&CachedPersistentState>,
    username: Option<&PlayerUsername>,
    persistence_ready: bool,
    rng: &mut impl Rng,
) {
    if !persistence_ready {
        return;
    }
    let Some(cache) = cache else {
        return;
    };
    let Some(username) = username else {
        return;
    };
    let progress = cache.weapon(cache.equipped_weapon_id);
    if let Some(transaction) = Transaction::death_penalty(
        username.0.clone(),
        cache.equipped_weapon_id,
        &progress,
        &settings.progression,
        rng,
    ) {
        transactions.write(transaction);
    }
}

fn tick_player_death(
    time: Res<Time>,
    settings: Res<GameSettings>,
    mut players: Query<
        (
            &mut PlayerPosition,
            &mut GameRoom,
            &mut PlayerHealth,
            &mut PlayerDeathTimer,
            &mut GateTeleportCooldown,
        ),
        With<ControlledBy>,
    >,
) {
    let safezone_center = settings.world.safezone_bounds().center();

    for (mut position, mut room, mut health, mut death_timer, mut gate_cooldown) in &mut players {
        if health.current > 0 || death_timer.remaining_seconds <= 0.0 {
            continue;
        }

        death_timer.remaining_seconds =
            (death_timer.remaining_seconds - time.delta_secs()).max(0.0);
        if death_timer.remaining_seconds > 0.0 {
            continue;
        }

        health.current = health.maximum;
        room.room = GameRooms::Safezone;
        position.0 = safezone_center;
        gate_cooldown.remaining_ticks = settings.gate.teleport_cooldown_ticks;
    }
}

#[cfg(feature = "gui")]
fn control_dedicated_server_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
    settings: Res<GameSettings>,
) {
    let (mut camera_transform, mut projection) = camera.into_inner();
    let Projection::Orthographic(ref mut orthographic) = *projection else {
        return;
    };

    let dt = time.delta_secs();

    let zoom_axis = if keyboard.pressed(KeyCode::Minus) {
        1.0
    } else if keyboard.pressed(KeyCode::Equal) {
        -1.0
    } else {
        0.0
    };

    if zoom_axis != 0.0 {
        orthographic.scale = (orthographic.scale
            * (settings.server_camera.zoom_speed * zoom_axis * dt).exp())
        .clamp(
            settings.server_camera.min_scale,
            settings.server_camera.max_scale,
        );
    }

    if keyboard.just_pressed(KeyCode::Digit0) {
        orthographic.scale = 1.0;
    }

    if keyboard.just_pressed(KeyCode::KeyF) {
        frame_server_world(&mut camera_transform, orthographic, &window, &settings);
    }

    let horizontal = axis(
        &keyboard,
        KeyCode::KeyA,
        KeyCode::KeyD,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
    );
    let vertical = axis(
        &keyboard,
        KeyCode::KeyS,
        KeyCode::KeyW,
        KeyCode::ArrowDown,
        KeyCode::ArrowUp,
    );

    let direction = Vec2::new(horizontal, vertical).normalize_or_zero();
    if direction != Vec2::ZERO {
        let visible_world_height = window.height() * orthographic.scale;
        camera_transform.translation += (direction
            * visible_world_height
            * settings.server_camera.pan_speed_in_screen_heights
            * dt)
            .extend(0.0);
    }
}

fn frame_server_world(
    camera_transform: &mut Transform,
    orthographic: &mut OrthographicProjection,
    window: &Window,
    settings: &GameSettings,
) {
    let arena_bounds = settings.world.arena_bounds();
    let safezone_bounds = settings.world.safezone_bounds();
    let combined_bounds = Rect {
        min: arena_bounds.min.min(safezone_bounds.min),
        max: arena_bounds.max.max(safezone_bounds.max),
    };

    const VIEW_PADDING: f32 = 1.08;
    let viewport_width = window.width().max(1.0);
    let viewport_height = window.height().max(1.0);

    let scale_for_width = combined_bounds.width() / viewport_width;
    let scale_for_height = combined_bounds.height() / viewport_height;

    orthographic.scale = scale_for_width.max(scale_for_height) * VIEW_PADDING;
    camera_transform.translation.x = combined_bounds.center().x;
    camera_transform.translation.y = combined_bounds.center().y;
}

fn axis(
    keyboard: &ButtonInput<KeyCode>,
    negative: KeyCode,
    positive: KeyCode,
    alternate_negative: KeyCode,
    alternate_positive: KeyCode,
) -> f32 {
    let negative_pressed = keyboard.pressed(negative) || keyboard.pressed(alternate_negative);
    let positive_pressed = keyboard.pressed(positive) || keyboard.pressed(alternate_positive);

    positive_pressed as i8 as f32 - negative_pressed as i8 as f32
}
