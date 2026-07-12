use bevy::{prelude::*, window::PrimaryWindow};
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

use crate::app::LocalUsername;
use crate::protocol::rooms::GameRoom;
use crate::{
    client::player::SmoothedAimDirection,
    protocol::{
        ClientEventsChannel, DebugServerMessage, PlayerAction, PlayerColor, PlayerId,
        PlayerPosition, SetUsername,
    },
    shared,
};

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct CachedCursorAim(pub Vec2);

impl Default for CachedCursorAim {
    fn default() -> Self {
        Self(Vec2::X)
    }
}

/// How quickly should the camera snap to the desired location.
const CAMERA_DECAY_RATE: f32 = 2.;

/// The client input only gets applied to predicted entities that we own
/// This works because we only predict the user's controlled entity.
/// If we were predicting more entities, we would have to only apply movement to the player owned one.
pub(crate) fn player_movement(
    synced_client: Query<(), (With<Client>, With<IsSynced<InputTimeline>>)>,
    mut player_query: Query<
        (&mut PlayerPosition, &GameRoom, &ActionState<PlayerAction>),
        With<Predicted>,
    >,
) {
    if synced_client.is_empty() {
        return;
    }

    for (position, room, actions) in player_query.iter_mut() {
        shared::shared_movement_behaviour(position, room, actions);
    }
}

/// Client-only system that smoothly moves the camera to the center of the Player's position.
pub(crate) fn update_camera(
    mut camera: Single<&mut Transform, With<Camera2d>>,
    player: Single<&PlayerPosition, With<Predicted>>,
    time: Res<Time>,
) {
    let target = player.0.extend(camera.translation.z);

    camera
        .translation
        .smooth_nudge(&target, CAMERA_DECAY_RATE, time.delta_secs());
}

pub(crate) fn receive_message1(mut receiver: Single<&mut MessageReceiver<DebugServerMessage>>) {
    for _message in receiver.receive() {
        //info!("Received message: {:?}", message);
    }
}

pub(crate) fn send_local_username(
    trigger: On<Add, Connected>,
    mut senders: Query<&mut MessageSender<SetUsername>>,
    username: Res<LocalUsername>,
) {
    if let Ok(mut sender) = senders.get_mut(trigger.entity) {
        sender.send::<ClientEventsChannel>(SetUsername {
            name: username.0.clone(),
        });
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
    mut commands: Commands,
    mut predicted: Query<(&mut PlayerColor, &PlayerId), With<Predicted>>,
) {
    let entity = trigger.entity;

    if let Ok((mut color, player_id)) = predicted.get_mut(entity) {
        commands
            .entity(entity)
            .insert(SmoothedAimDirection::default())
            .insert(Name::new(format!("Player (Predicted): {:?}", player_id.0)));

        let hsva = Hsva {
            saturation: 0.4,
            ..Hsva::from(color.0)
        };

        color.0 = Color::from(hsva);
    }
}

pub(crate) fn sample_cursor_aim(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    predicted_player: Single<&PlayerPosition, With<Predicted>>,
    mut input_entity: Single<
        &mut CachedCursorAim,
        (With<Controlled>, With<InputMap<PlayerAction>>),
    >,
) {
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let (camera, camera_transform) = camera.into_inner();

    let Ok(cursor_world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position)
    else {
        return;
    };

    let direction = (cursor_world_position - predicted_player.0).normalize_or_zero();

    if direction == Vec2::ZERO {
        return;
    }

    input_entity.0 = direction;
}

pub(crate) fn write_cursor_aim_to_leafwing(
    input_entity: Single<
        (&CachedCursorAim, &mut ActionState<PlayerAction>),
        (With<Controlled>, With<InputMap<PlayerAction>>),
    >,
) {
    let (cached_aim, mut actions) = input_entity.into_inner();

    actions.set_axis_pair(&PlayerAction::Aim, cached_aim.0);
}
