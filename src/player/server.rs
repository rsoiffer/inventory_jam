use crate::app::ServerState;
use crate::enemy::{EnemyHealth, EnemyKind, EnemyPosition};
use crate::player::PlayerId;
use crate::player::api::AddKillsToAristeia;
use crate::player::protocol::{
    PlayerAimDirection, PlayerAristeia, PlayerBundle, PlayerHealth, PlayerPosition, PlayerVisual,
};
use crate::player::shared::{apply_player_aim, apply_player_movement};
use crate::portal::PortalTeleportCooldown;
use crate::protocol::inputs::PlayerAction;
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::settings::GameSettings;
use crate::shared::FixedGameplaySet;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use leafwing_input_manager::prelude::*;
use lightyear::connection::client::Connected;
use lightyear::connection::client_of::ClientOf;
use lightyear::prelude::*;

#[derive(Component, Clone, Copy, Debug, Default)]
struct EnemyContactDamageCooldown {
    remaining_seconds: f32,
}

#[derive(Component, Clone, Copy, Debug, Default)]
struct PlayerDeathTimer {
    remaining_seconds: f32,
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
                restore_safezone_health,
                apply_enemy_contact_damage,
                tick_player_death,
                control_dedicated_server_camera,
                tick_player_aristeia,
            )
                .chain()
                .in_set(FixedGameplaySet::Player)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

pub(crate) fn add_requested_aristeia(
    settings: Res<GameSettings>,
    mut requests: MessageReader<AddKillsToAristeia>,
    mut players: Query<(&PlayerId, &mut PlayerAristeia)>,
) {
    for AddKillsToAristeia(owner_id, number_of_kills) in requests.read() {
        let Some(mut aristeia) = players
            .iter_mut()
            .find_map(|(player_id, aristeia)| (player_id.0 == *owner_id).then_some(aristeia))
        else {
            warn!(
                ?owner_id,
                number_of_kills, "No player matched the Aristeia award"
            );
            continue;
        };

        aristeia.current = aristeia.current.saturating_add(u32::from(*number_of_kills));

        aristeia.maximum_ticks = settings.player.aristeia_duration_ticks;

        aristeia.remaining_ticks = settings.player.aristeia_duration_ticks;
    }
}

fn tick_player_aristeia(mut players: Query<&mut PlayerAristeia>) {
    for mut aristeia in &mut players {
        if aristeia.current == 0 {
            aristeia.remaining_ticks = 0;
            continue;
        }

        if aristeia.remaining_ticks > 0 {
            aristeia.remaining_ticks -= 1;
            continue;
        }

        aristeia.current = aristeia.current.saturating_sub(1);

        if aristeia.current > 0 {
            aristeia.remaining_ticks = aristeia.maximum_ticks;
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
                settings.player.aristeia_duration_ticks,
                settings.projectile.buffer_capacity,
            ),
            PlayerVisual,
            EnemyContactDamageCooldown::default(),
            PlayerDeathTimer::default(),
            PortalTeleportCooldown {
                remaining_ticks: settings.portal.teleport_cooldown_ticks,
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
            Has<Predicted>,
            &mut PlayerPosition,
            &GameRoom,
            &PlayerHealth,
            &ActionState<PlayerAction>,
        ),
        With<PlayerId>,
    >,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let skip_predicted = !host_server.is_empty();

    for (predicted, mut position, room, health, actions) in &mut players {
        if skip_predicted && predicted {
            continue;
        }

        apply_player_movement(&mut position, room, health, actions, &settings);
    }
}

pub(crate) fn authoritative_player_aim(
    mut players: Query<
        (
            Has<Predicted>,
            &ActionState<PlayerAction>,
            &mut PlayerAimDirection,
        ),
        With<PlayerId>,
    >,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let skip_predicted = !host_server.is_empty();

    for (predicted, actions, mut aim_direction) in &mut players {
        if skip_predicted && predicted {
            continue;
        }

        apply_player_aim(&mut aim_direction, actions);
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

fn apply_enemy_contact_damage(
    time: Res<Time>,
    mut players: Query<
        (
            Entity,
            &PlayerPosition,
            &GameRoom,
            &mut PlayerHealth,
            &mut EnemyContactDamageCooldown,
            &mut PlayerDeathTimer,
        ),
        With<ControlledBy>,
    >,
    enemies: Query<(&EnemyPosition, &EnemyHealth, &EnemyKind)>,
    settings: Res<GameSettings>,
) {
    for (player_entity, player_position, room, mut health, mut cooldown, mut death_timer) in
        &mut players
    {
        cooldown.remaining_seconds = (cooldown.remaining_seconds - time.delta_secs()).max(0.0);

        if room.room != GameRooms::Arena || health.current == 0 || cooldown.remaining_seconds > 0.0
        {
            continue;
        }

        let touching_enemy = enemies.iter().any(|(enemy_position, enemy_health, kind)| {
            if enemy_health.current == 0 {
                return false;
            }

            let collision_radius = settings.player.collision_radius
                + kind.collision_radius(settings.enemy.collision_radius);
            player_position.0.distance_squared(enemy_position.0)
                <= collision_radius * collision_radius
        });

        if !touching_enemy {
            continue;
        }

        health.current = health
            .current
            .saturating_sub(settings.player.enemy_contact_damage);
        cooldown.remaining_seconds = settings.player.enemy_contact_damage_interval_seconds;

        info!(
            ?player_entity,
            damage = settings.player.enemy_contact_damage,
            current_health = health.current,
            maximum_health = health.maximum,
            "Player took enemy contact damage"
        );

        if health.current == 0 {
            death_timer.remaining_seconds = settings.player.death_screen_duration_seconds;
            info!(?player_entity, "Player died");
        }
    }
}

fn tick_player_death(
    time: Res<Time>,
    settings: Res<GameSettings>,
    mut players: Query<
        (
            Entity,
            &mut PlayerPosition,
            &mut GameRoom,
            &mut PlayerHealth,
            &mut PlayerDeathTimer,
            &mut PortalTeleportCooldown,
        ),
        With<ControlledBy>,
    >,
) {
    let safezone_center = settings.world.safezone_bounds().center();

    for (player_entity, mut position, mut room, mut health, mut death_timer, mut portal_cooldown) in
        &mut players
    {
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
        portal_cooldown.remaining_ticks = settings.portal.teleport_cooldown_ticks;
        info!(?player_entity, "Player respawned in the pit");
    }
}

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
