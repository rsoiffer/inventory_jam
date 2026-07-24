use crate::app::ServerState;
use crate::enemy::{EnemyHealth, EnemyPosition};
use crate::player::api::AddKillsToAristeia;
use crate::player::protocol::{
    PlayerAimDirection, PlayerAristeia, PlayerBundle, PlayerHealth, PlayerPosition, PlayerVisual,
};
use crate::player::shared::player_movement;
use crate::player::PlayerId;
use crate::protocol::inputs::PlayerAction;
use crate::protocol::messages::TeleportRoom;
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::settings::GameSettings;
use crate::shared::FixedGameplaySet;
use bevy::prelude::*;
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

/// Controls where newly connected players appear.
///
/// `Annulus` chooses a random point between `minimum_radius` and
/// `maximum_radius` from `center`. Set both radii to the same value to spawn at
/// an exact distance from the center.
///
/// `Fixed` always uses the requested position, clamped inside the room bounds.
#[derive(Resource, Clone, Copy, Debug)]
pub enum PlayerSpawnMode {
    Annulus {
        center: Vec2,
        minimum_radius: f32,
        maximum_radius: f32,
    },
    #[allow(unused)]
    Fixed(Vec2),
}

impl Default for PlayerSpawnMode {
    fn default() -> Self {
        Self::Annulus {
            center: Vec2::ZERO,
            minimum_radius: 56.0,
            maximum_radius: 80.0,
        }
    }
}

pub struct PlayerServerPlugin;

impl Plugin for PlayerServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSpawnMode>();
        app.add_observer(handle_connected);

        app.add_systems(
            FixedUpdate,
            (
                player_movement,
                update_player_aim_direction,
                add_requested_aristeia,
                apply_enemy_contact_damage,
                control_dedicated_server_camera,
                tick_player_aristeia,
            )
                .chain()
                .in_set(FixedGameplaySet::Player)
                .run_if(in_state(ServerState::Hosting)),
        );

        app.add_systems(
            FixedUpdate,
            apply_teleport_room_commands.run_if(in_state(ServerState::Hosting)),
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
    spawn_mode: Res<PlayerSpawnMode>,
    settings: Res<GameSettings>,
    mut commands: Commands,
) {
    let Ok(client_id) = query.get(trigger.entity) else {
        return;
    };

    let client_id = client_id.0;
    let room = GameRoom {
        room: GameRooms::Arena,
    };
    let spawn_position = choose_player_spawn(*spawn_mode, room, &settings);

    let entity = commands
        .spawn((
            Name::new(format!("Player: {}", client_id)),
            PlayerBundle::new(
                client_id,
                spawn_position,
                settings.player.aristeia_duration_ticks,
            ),
            PlayerVisual,
            EnemyContactDamageCooldown::default(),
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

fn choose_player_spawn(mode: PlayerSpawnMode, room: GameRoom, settings: &GameSettings) -> Vec2 {
    let bounds = room
        .bounds(&settings.world)
        .inflate(-settings.player.half_size);

    match mode {
        PlayerSpawnMode::Fixed(position) => position.clamp(bounds.min, bounds.max),
        PlayerSpawnMode::Annulus {
            center,
            minimum_radius,
            maximum_radius,
        } => {
            let minimum_radius = minimum_radius.max(0.0);
            let maximum_radius = maximum_radius.max(minimum_radius);
            let mut rng = rand::rng();

            for _ in 0..settings.player.spawn_attempts {
                let angle = rng.random_range(0.0..std::f32::consts::TAU);
                let radius_squared = rng.random_range(
                    minimum_radius * minimum_radius..=maximum_radius * maximum_radius,
                );
                let radius = radius_squared.sqrt();
                let candidate = center + Vec2::new(angle.cos(), angle.sin()) * radius;

                if bounds.contains(candidate) {
                    return candidate;
                }
            }

            // A malformed or impossible annulus still produces a valid position.
            center.clamp(bounds.min, bounds.max)
        }
    }
}

pub(crate) fn update_player_aim_direction(
    mut players: Query<(&ActionState<PlayerAction>, &mut PlayerAimDirection)>,
) {
    for (actions, mut aim_direction) in &mut players {
        let aim = actions.clamped_axis_pair(&PlayerAction::Aim);

        if aim.length_squared() > 0.0001 {
            aim_direction.0 = aim.normalize_or_zero();
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
        ),
        With<ControlledBy>,
    >,
    enemies: Query<(&EnemyPosition, &EnemyHealth)>,
    settings: Res<GameSettings>,
) {
    for (player_entity, player_position, room, mut health, mut cooldown) in &mut players {
        cooldown.remaining_seconds = (cooldown.remaining_seconds - time.delta_secs()).max(0.0);

        if room.room != GameRooms::Arena || health.current == 0 || cooldown.remaining_seconds > 0.0
        {
            continue;
        }

        let touching_enemy = enemies.iter().any(|(enemy_position, enemy_health)| {
            if enemy_health.current == 0 {
                return false;
            }

            let collision_radius =
                settings.player.collision_radius + settings.enemy.collision_radius;
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
    }
}

fn apply_teleport_room_commands(
    mut receivers: Query<(Entity, &mut MessageReceiver<TeleportRoom>), With<ClientOf>>,
    mut players: Query<(&ControlledBy, &mut GameRoom, &mut PlayerPosition)>,
    settings: Res<GameSettings>,
) {
    let mut rng = rand::rng();

    for (link_entity, mut receiver) in &mut receivers {
        for _ in receiver.receive() {
            let Some((mut room, mut position)) = players
                .iter_mut()
                .find(|(controlled_by, _, _)| controlled_by.owner == link_entity)
                .map(|(_, room, position)| (room, position))
            else {
                warn!("no authoritative player for TeleportRoom from {link_entity:?}");
                continue;
            };

            room.room = match room.room {
                GameRooms::Arena => GameRooms::Safezone,
                GameRooms::Safezone => GameRooms::Arena,
            };
            position.0 = match room.room {
                GameRooms::Arena => random_position_in_room(*room, &settings, &mut rng),
                GameRooms::Safezone => settings.world.safezone_bounds().center(),
            };
        }
    }
}

fn random_position_in_room(room: GameRoom, settings: &GameSettings, rng: &mut impl Rng) -> Vec2 {
    let bounds = room
        .bounds(&settings.world)
        .inflate(-settings.player.half_size);
    Vec2::new(
        rng.random_range(bounds.min.x..=bounds.max.x),
        rng.random_range(bounds.min.y..=bounds.max.y),
    )
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

#[allow(unused)]
fn draw_world_boundaries(settings: Res<GameSettings>, mut gizmos: Gizmos) {
    for bounds in [
        settings.world.arena_bounds(),
        settings.world.safezone_bounds(),
    ] {
        gizmos.rect_2d(
            Isometry2d::from_translation(bounds.center()),
            bounds.size(),
            Color::srgb(0.95, 0.25, 0.2),
        );
    }
}
