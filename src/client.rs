//! The client plugin.
//! The client will be responsible for:
//! - connecting to the server at Startup
//! - sending inputs to the server
//! - applying inputs to the locally predicted player (for prediction to work, inputs have to be applied to both the
//!   predicted entity and the server entity)

use crate::protocol::{Direction, Inputs, Message1, PlayerColor, PlayerId, PlayerPosition};
use crate::shared;
use bevy::prelude::{
    Add, App, Color, Commands, Entity, FixedPreUpdate, FixedUpdate, Hsva, IntoScheduleConfigs,
    KeyCode, On, Plugin, Query, Reflect, Res, Single, Startup, Update, With, Without, info, trace,
};
use leafwing_input_manager::Actionlike;
use leafwing_input_manager::input_map::InputMap;
use leafwing_input_manager::prelude::ActionState as LeafwingActionState;
use leafwing_input_manager::prelude::InputManagerPlugin;
use lightyear::prelude::client::input::*;
use lightyear::prelude::client::{InputDelayConfig, InputTimelineConfig};
use lightyear::prelude::input::native::ActionState as LightyearActionState;
use lightyear::prelude::input::native::InputMarker;
use lightyear::prelude::{
    Client, Controlled, ControlledBy, InputTimeline, Interpolated, IsSynced, LocalTimeline,
    MessageReceiver, Predicted,
};

pub struct ExampleClientPlugin;

impl Plugin for ExampleClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, configure_input_delay);
        app.add_systems(
            FixedPreUpdate,
            // Inputs have to be buffered in the WriteClientInputs set
            buffer_input.in_set(InputSystems::WriteClientInputs),
        );
        app.add_systems(FixedUpdate, player_movement);

        app.add_systems(Update, receive_message1);
        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_controlled_spawn);
        app.add_observer(handle_interpolated_spawn);
        app.add_plugins(InputManagerPlugin::<Action>::default());
    }
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
enum Action {
    Up,
    Down,
    Left,
    Right,
}

fn configure_input_delay(client: Single<Entity, With<Client>>, mut commands: Commands) {
    commands.entity(client.into_inner()).insert(
        InputTimelineConfig::default().with_input_delay(InputDelayConfig::no_input_delay()),
    );
}

/// System that reads from peripherals and adds inputs to the buffer
/// This system must be run in the `InputSystemSet::BufferInputs` set in the `FixedPreUpdate` schedule
/// to work correctly.
///
/// I would also advise to use the `leafwing` feature to use the `LeafwingInputPlugin` instead of the
/// `InputPlugin`, which contains more features.
fn buffer_input(
    timeline: Res<LocalTimeline>,
    mut query: Query<
        (
            &mut LightyearActionState<Inputs>,
            &LeafwingActionState<Action>,
        ),
        With<InputMarker<Inputs>>,
    >,
) {
    if let Ok((mut lightyear_action, leaf_action)) = query.single_mut() {
        let current_tick = timeline.tick();
        let mut direction = Direction::default();

        if leaf_action.pressed(&Action::Up) {
            direction.up = true;
        }
        if leaf_action.pressed(&Action::Down) {
            direction.down = true;
        }
        if leaf_action.pressed(&Action::Left) {
            direction.left = true;
        }
        if leaf_action.pressed(&Action::Right) {
            direction.right = true;
        }

        // we always set the value. Setting it to None means that the input was missing, it's not the same
        // as saying that the input was 'no keys pressed'
        lightyear_action.0 = Inputs { direction };
        trace!(
            target: "lightyear_debug::simple_box",
            kind = "simple_box_client_input",
            schedule = "FixedPreUpdate",
            sample_point = "FixedPreUpdate",
            local_tick = current_tick.0,
            input = ?lightyear_action.0,
            "selected simple_box client input"
        );
    }
}

/// The client input only gets applied to predicted entities that we own
/// This works because we only predict the user's controlled entity.
/// If we were predicting more entities, we would have to only apply movement to the player owned one.
fn player_movement(
    synced_client: Query<(), (With<Client>, With<IsSynced<InputTimeline>>)>,
    // timeline: Single<&LocalTimeline>,
    mut position_query: Query<
        (&mut PlayerPosition, &LightyearActionState<Inputs>),
        With<Predicted>,
    >,
) {
    if synced_client.is_empty() {
        return;
    }
    // let tick = timeline.tick();
    for (position, input) in position_query.iter_mut() {
        // trace!(?tick, ?position, ?input, "client");
        // NOTE: be careful to directly pass Mut<PlayerPosition>
        // getting a mutable reference triggers change detection, unless you use `as_deref_mut()`
        shared::shared_movement_behaviour(position, input);
    }
}

/// System to receive messages on the client
pub(crate) fn receive_message1(mut receiver: Single<&mut MessageReceiver<Message1>>) {
    for message in receiver.receive() {
        info!("Received message: {:?}", message);
    }
}

/// When the predicted copy of the client-owned entity is spawned, do stuff
/// - assign it a different saturation
/// - keep track of it in the Global resource
///
/// Note that this will be triggered multiple times: for the locally-controlled entity,
/// but also for the remote-controlled entities that are spawned with [`Interpolated`].
/// The `With<Predicted>` filter ensures we only add the `InputMarker` once.
pub(crate) fn handle_predicted_spawn(
    trigger: On<Add, (PlayerId, Predicted)>,
    mut predicted: Query<&mut PlayerColor, With<Predicted>>,
) {
    let entity = trigger.entity;
    if let Ok(mut color) = predicted.get_mut(entity) {
        let hsva = Hsva {
            saturation: 0.4,
            ..Hsva::from(color.0)
        };
        color.0 = Color::from(hsva);
    }
}

fn handle_controlled_spawn(
    trigger: On<Add, Controlled>,
    mut commands: Commands,
    players: Query<(&PlayerId, Option<&ControlledBy>), Without<InputMarker<Inputs>>>,
    clients: Query<(), With<Client>>,
) {
    let entity = trigger.entity;
    let Ok((player_id, controlled_by)) = players.get(entity) else {
        return;
    };
    if let Some(controlled_by) = controlled_by
        && clients.get(controlled_by.owner).is_err()
    {
        return;
    }
    info!("Adding InputMarker to controlled player {entity:?} {player_id:?}");
    let input_map = InputMap::new([
        (Action::Up, KeyCode::ArrowUp),
        (Action::Down, KeyCode::ArrowDown),
        (Action::Left, KeyCode::ArrowLeft),
        (Action::Right, KeyCode::ArrowRight),
    ]);
    commands
        .entity(entity)
        .insert(input_map)
        .insert(InputMarker::<Inputs>::default());
}

/// When the predicted copy of the client-owned entity is spawned, do stuff
/// - assign it a different saturation
/// - keep track of it in the Global resource
pub(crate) fn handle_interpolated_spawn(
    trigger: On<Add, Interpolated>,
    mut interpolated: Query<&mut PlayerColor>,
) {
    if let Ok(mut color) = interpolated.get_mut(trigger.entity) {
        let hsva = Hsva {
            saturation: 0.1,
            ..Hsva::from(color.0)
        };
        color.0 = Color::from(hsva);
    }
}
