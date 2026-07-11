use bevy::prelude::*;

pub const FRAGMENT_RADIUS: f32 = 8.0;

/// How long an uncollected fragment remains in the world at 60 fixed ticks/sec.
pub const FRAGMENT_LIFETIME_TICKS: u16 = 600;

/// How long the post-award pull visual remains before the server despawns it.
pub const FRAGMENT_PICKUP_VISUAL_TICKS: u16 = 30;

/// Radius owned by the player for collecting nearby fragments.
pub const PLAYER_COLLECTION_RADIUS: f32 = 50.0;

/// Fraction of the remaining distance covered each fixed tick during pickup.
const FRAGMENT_PULL_FRACTION_PER_TICK: f32 = 0.22;

/// Sideways motion added while pulling, producing a drain-like curve.
const FRAGMENT_SWIRL_SPEED_PER_TICK: f32 = 1.5;

#[derive(Component, Clone, Copy, Debug, PartialEq, Deref, DerefMut)]
pub struct FragmentPosition(pub Vec2);

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct FragmentLifetime {
    pub remaining_ticks: u16,
}

/// Marker for the authoritative server-side fragments.
#[derive(Component)]
pub struct ServerFragment;

/// Marker for a client fragments.
#[derive(Component)]
pub struct ClientFragment;

/// Moves a fragment toward a player while retaining a curved, orbit-like path.
pub fn pull_fragment_toward(position: &mut FragmentPosition, target: Vec2) {
    let to_target = target - position.0;
    let distance = to_target.length();

    if distance <= f32::EPSILON {
        position.0 = target;
        return;
    }

    let radial_step = to_target * FRAGMENT_PULL_FRACTION_PER_TICK;
    let tangent = Vec2::new(-to_target.y, to_target.x).normalize_or_zero();
    let swirl_scale = (distance / PLAYER_COLLECTION_RADIUS).clamp(0.0, 1.0);
    let swirl_step = tangent * FRAGMENT_SWIRL_SPEED_PER_TICK * swirl_scale;
    let step = radial_step + swirl_step;

    if step.length_squared() >= distance * distance {
        position.0 = target;
    } else {
        position.0 += step;
    }
}
