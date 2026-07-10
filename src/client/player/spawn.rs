//! Client-side spawn observers for interpolated players.

use bevy::prelude::*;
use lightyear::prelude::*;

use crate::protocol::PlayerColor;

/// Desaturates interpolated remote players so they read differently from predicted ones.
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
