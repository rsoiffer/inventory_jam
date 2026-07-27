//! Touchscreen controls for the web client: movement joystick and nearest-enemy auto-fire.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use leafwing_input_manager::action_state::ActionState;
use leafwing_input_manager::input_map::InputMap;
use lightyear::input::client::InputSystems;
use lightyear::prelude::{Controlled, Predicted};
use virtual_joystick::{
    JoystickDeadZone, JoystickFixed, NoAction, VirtualJoystickMessage, VirtualJoystickMessageType,
    VirtualJoystickNode, VirtualJoystickPlugin, create_joystick,
};

use crate::app::ClientState;
use crate::enemy::{EnemyHealth, EnemyPosition};
use crate::persistence::CachedPersistentState;
use crate::player::nearest_enemy_aim::aim_and_fire_nearest_enemy;
use crate::player::protocol::{PlayerHealth, PlayerPosition};
use crate::protocol::inputs::PlayerAction;
use crate::protocol::rooms::GameRoom;
use crate::settings::GameSettings;

const MOVE_JOYSTICK_ID: &str = "move";

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TouchControlsEnabled(pub bool);

#[derive(Resource, Default, Clone, Copy, Debug)]
struct JoystickMoveInput(Vec2);

#[derive(Resource, Clone, Debug)]
struct JoystickImages {
    knob: Handle<Image>,
    background: Handle<Image>,
}

pub struct MobileControlsPlugin;

impl Plugin for MobileControlsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TouchControlsEnabled(detect_touch_controls()))
            .init_resource::<JoystickMoveInput>()
            .add_plugins(VirtualJoystickPlugin::<String>::default())
            .add_systems(Startup, load_joystick_images)
            .add_systems(
                OnEnter(ClientState::Playing),
                spawn_move_joystick.run_if(touch_controls_enabled),
            )
            .add_systems(
                OnExit(ClientState::Playing),
                despawn_move_joystick.run_if(touch_controls_enabled),
            )
            .add_systems(
                Update,
                read_joystick_messages
                    .run_if(in_state(ClientState::Playing))
                    .run_if(touch_controls_enabled),
            )
            .add_systems(
                FixedPreUpdate,
                write_touch_actions_to_leafwing
                    .in_set(InputSystems::WriteClientInputs)
                    .run_if(in_state(ClientState::Playing))
                    .run_if(touch_controls_enabled),
            );
    }
}

fn touch_controls_enabled(enabled: Res<TouchControlsEnabled>) -> bool {
    enabled.0
}

fn detect_touch_controls() -> bool {
    #[cfg(target_family = "wasm")]
    {
        let Some(window) = web_sys::window() else {
            return false;
        };

        if window
            .match_media("(pointer: coarse)")
            .ok()
            .flatten()
            .is_some_and(|query| query.matches())
        {
            return true;
        }

        window.navigator().max_touch_points() > 0
    }

    #[cfg(not(target_family = "wasm"))]
    {
        false
    }
}

fn load_joystick_images(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let knob = images.add(circle_image(64, [255, 255, 255, 210], false));
    let background = images.add(circle_image(128, [255, 255, 255, 90], true));
    commands.insert_resource(JoystickImages { knob, background });
}

fn circle_image(diameter: u32, rgba: [u8; 4], ring: bool) -> Image {
    let mut data = vec![0_u8; (diameter * diameter * 4) as usize];
    let radius = diameter as f32 * 0.5;
    let center = radius - 0.5;
    let outer = radius - 1.0;
    let inner = (radius - 5.0).max(0.0);

    for y in 0..diameter {
        for x in 0..diameter {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let alpha = if ring {
                if distance <= outer && distance >= inner {
                    rgba[3]
                } else {
                    0
                }
            } else if distance <= outer {
                rgba[3]
            } else {
                0
            };

            let index = ((y * diameter + x) * 4) as usize;
            data[index] = rgba[0];
            data[index + 1] = rgba[1];
            data[index + 2] = rgba[2];
            data[index + 3] = alpha;
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: diameter,
            height: diameter,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    image
}

fn spawn_move_joystick(
    mut commands: Commands,
    settings: Res<GameSettings>,
    images: Res<JoystickImages>,
) {
    let size = settings.mobile_controls.joystick_size;
    let knob = settings.mobile_controls.knob_size;
    let inset = settings.mobile_controls.screen_inset;
    let dead_zone = settings.mobile_controls.movement_dead_zone;

    create_joystick(
        &mut commands,
        MOVE_JOYSTICK_ID.to_owned(),
        images.knob.clone(),
        images.background.clone(),
        Some(Color::srgba(0.95, 0.95, 1.0, 0.85)),
        Some(Color::srgba(0.85, 0.9, 1.0, 0.55)),
        Some(Color::srgba(0.05, 0.08, 0.12, 0.12)),
        Vec2::splat(knob),
        Vec2::splat(size),
        Node {
            width: Val::Px(size * 1.6),
            height: Val::Px(size * 1.6),
            position_type: PositionType::Absolute,
            left: Val::Px(inset),
            bottom: Val::Px(inset),
            ..default()
        },
        (JoystickFixed, JoystickDeadZone(dead_zone)),
        NoAction,
    );
}

fn despawn_move_joystick(
    mut commands: Commands,
    joysticks: Query<Entity, With<VirtualJoystickNode<String>>>,
    mut move_input: ResMut<JoystickMoveInput>,
) {
    move_input.0 = Vec2::ZERO;
    for entity in &joysticks {
        commands.entity(entity).despawn();
    }
}

fn read_joystick_messages(
    mut reader: MessageReader<VirtualJoystickMessage<String>>,
    mut move_input: ResMut<JoystickMoveInput>,
) {
    for message in reader.read() {
        if message.id() != MOVE_JOYSTICK_ID {
            continue;
        }

        move_input.0 = match message.get_type() {
            VirtualJoystickMessageType::Up => Vec2::ZERO,
            VirtualJoystickMessageType::Press | VirtualJoystickMessageType::Drag => *message.axis(),
        };
    }
}

type TouchControlledPlayer<'w> = (
    &'w PlayerPosition,
    &'w GameRoom,
    &'w PlayerHealth,
    &'w CachedPersistentState,
    Mut<'w, ActionState<PlayerAction>>,
);

fn write_touch_actions_to_leafwing(
    move_input: Res<JoystickMoveInput>,
    settings: Res<GameSettings>,
    mut players: Query<
        TouchControlledPlayer,
        (
            With<Controlled>,
            With<Predicted>,
            With<InputMap<PlayerAction>>,
        ),
    >,
    enemies: Query<(&EnemyPosition, &EnemyHealth, &GameRoom)>,
) {
    for (player_position, player_room, health, cache, mut actions) in &mut players {
        if move_input.0 != Vec2::ZERO {
            actions.set_axis_pair(&PlayerAction::Move, move_input.0);
        }

        aim_and_fire_nearest_enemy(
            &mut actions,
            player_position.0,
            player_room,
            health.current,
            cache,
            &settings,
            &enemies,
        );
    }
}
