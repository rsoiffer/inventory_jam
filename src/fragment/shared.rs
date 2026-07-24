use bevy::prelude::*;

use crate::settings::FragmentSettings;

#[derive(Component, Clone, Copy, Debug, PartialEq, Deref, DerefMut)]
pub struct FragmentPosition(pub Vec2);

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct FragmentLifetime {
    pub remaining_ticks: u16,
}

/// Ticks since this fragment started magnetizing toward its collector.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FragmentMagnetAge(pub u16);

/// Marker for the authoritative server-side fragments.
#[derive(Component)]
pub struct ServerFragment;

/// Marker for client fragments.
#[derive(Component)]
pub struct ClientFragment;

/// Moves a fragment toward a magnet target at accelerating speed, then swirls
/// around that target by `swirl * move_distance / distance` radians.
///
/// `pull_speed` is world-units per second; `pull_acceleration` is world-units
/// per second squared.
pub fn pull_fragment_toward(
    position: &mut FragmentPosition,
    target: Vec2,
    magnet_age: u16,
    tick_secs: f32,
    settings: &FragmentSettings,
) {
    let to_target = target - position.0;
    let distance = to_target.length();
    if distance <= f32::EPSILON {
        position.0 = target;
        return;
    }

    let age_secs = magnet_age as f32 * tick_secs;
    let speed = settings.pull_speed + settings.pull_acceleration * age_secs;
    let move_distance = (speed * tick_secs).min(distance);
    let after_radial = position.0 + to_target * (move_distance / distance);

    let angle = settings.swirl * move_distance / distance;
    let (sin, cos) = angle.sin_cos();
    let offset = after_radial - target;
    position.0 = target
        + Vec2::new(
            offset.x * cos - offset.y * sin,
            offset.x * sin + offset.y * cos,
        );
}
