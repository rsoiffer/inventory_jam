use bevy::prelude::*;

use crate::settings::FragmentSettings;

#[derive(Component, Clone, Copy, Debug, PartialEq, Deref, DerefMut)]
pub struct FragmentPosition(pub Vec2);

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct FragmentLifetime {
    pub remaining_ticks: u16,
}

/// Marker for the authoritative server-side fragments.
#[derive(Component)]
pub struct ServerFragment;

/// Marker for client fragments.
#[derive(Component)]
pub struct ClientFragment;

/// Moves a fragment toward a player while retaining a curved, orbit-like path.
pub fn pull_fragment_toward(
    position: &mut FragmentPosition,
    target: Vec2,
    settings: &FragmentSettings,
) {
    let to_target = target - position.0;
    let distance = to_target.length();

    if distance <= f32::EPSILON {
        position.0 = target;
        return;
    }

    let radial_step = to_target * settings.pull_fraction_per_tick;
    let tangent = Vec2::new(-to_target.y, to_target.x).normalize_or_zero();
    let swirl_scale = (distance / settings.player_collection_radius).clamp(0.0, 1.0);
    let swirl_step = tangent * settings.swirl_speed_per_tick * swirl_scale;
    let step = radial_step + swirl_step;

    if step.length_squared() >= distance * distance {
        position.0 = target;
    } else {
        position.0 += step;
    }
}
