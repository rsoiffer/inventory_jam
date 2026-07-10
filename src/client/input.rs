use bevy::prelude::*;
use lightyear::prelude::{client::InputDelayConfig, input::native::{ActionState, InputMarker}, *};
use crate::protocol::{Direction, Inputs};

pub(crate) fn configure_input_delay(client: Single<Entity, With<Client>>, mut commands: Commands) {
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
pub(crate) fn buffer_input(
    timeline: Res<LocalTimeline>,
    mut query: Query<&mut ActionState<Inputs>, With<InputMarker<Inputs>>>,
    keypress: Option<Res<ButtonInput<KeyCode>>>,
) {
    if let Ok(mut action_state) = query.single_mut() {
        let current_tick = timeline.tick();
        let mut direction = Direction::default();

        if let Some(keypress) = keypress {
            if keypress.pressed(KeyCode::KeyW) || keypress.pressed(KeyCode::ArrowUp) {
                direction.up = true;
            }
            if keypress.pressed(KeyCode::KeyS) || keypress.pressed(KeyCode::ArrowDown) {
                direction.down = true;
            }
            if keypress.pressed(KeyCode::KeyA) || keypress.pressed(KeyCode::ArrowLeft) {
                direction.left = true;
            }
            if keypress.pressed(KeyCode::KeyD) || keypress.pressed(KeyCode::ArrowRight) {
                direction.right = true;
            }
        }

        // we always set the value. Setting it to None means that the input was missing, it's not the same
        // as saying that the input was 'no keys pressed'
        action_state.0 = Inputs::Direction(direction);
        trace!(
            target: "lightyear_debug::simple_box",
            kind = "simple_box_client_input",
            schedule = "FixedPreUpdate",
            sample_point = "FixedPreUpdate",
            local_tick = current_tick.0,
            input = ?action_state.0,
            "selected simple_box client input"
        );
    }
}
