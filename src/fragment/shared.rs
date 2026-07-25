use bevy::prelude::*;
use lightyear::prelude::{PeerId, Tick};

use crate::fragment::protocol::{Fragment, FragmentPhase};
use crate::settings::FragmentSettings;

/// Prefer the predicted player copy when present; otherwise use any match.
pub fn collector_position(
    collector: PeerId,
    players: impl IntoIterator<Item = (PeerId, Vec2, bool)>,
) -> Option<Vec2> {
    let mut fallback = None;
    for (id, position, predicted) in players {
        if id != collector {
            continue;
        }
        if predicted {
            return Some(position);
        }
        fallback = Some(position);
    }
    fallback
}

/// Shared per-tick fragment motion / phase advance used by both client and server.
pub fn tick_fragment(
    fragment: &mut Fragment,
    target: Option<Vec2>,
    current_tick: Tick,
    tick_secs: f32,
    settings: &FragmentSettings,
) {
    let age = |start_tick: Tick| {
        current_tick
            .0
            .wrapping_sub(start_tick.0)
            .min(u16::MAX as u32) as u16
    };

    match fragment.phase {
        FragmentPhase::OnGround | FragmentPhase::Gone => {
            fragment.movement = Vec2::ZERO;
        }
        FragmentPhase::StartingPull {
            collector,
            start_tick,
        } => {
            fragment.movement = Vec2::ZERO;
            let pull_ticks = ((settings.starting_pull.frames as f32
                * settings.starting_pull.frame_seconds)
                / tick_secs.max(f32::EPSILON))
            .ceil() as u16;
            if age(start_tick) >= pull_ticks.max(1) {
                fragment.phase = FragmentPhase::Moving {
                    collector,
                    start_tick: current_tick,
                };
            }
        }
        FragmentPhase::Moving {
            collector,
            start_tick,
        } => {
            let Some(target) = target else {
                fragment.movement = Vec2::ZERO;
                return;
            };

            let previous = fragment.position;
            let to_target = target - fragment.position;
            let distance = to_target.length();
            if distance <= f32::EPSILON {
                fragment.position = target;
                fragment.movement = Vec2::ZERO;
            } else {
                let speed = settings.pull_speed
                    + settings.pull_acceleration * age(start_tick) as f32 * tick_secs;
                let move_distance = (speed * tick_secs).min(distance);
                let after_radial = fragment.position + to_target * (move_distance / distance);
                let angle = settings.swirl * move_distance / distance;
                let (sin, cos) = angle.sin_cos();
                let offset = after_radial - target;
                fragment.position = target
                    + Vec2::new(
                        offset.x * cos - offset.y * sin,
                        offset.x * sin + offset.y * cos,
                    );
                fragment.movement = fragment.position - previous;
            }

            if fragment.position.distance_squared(target) <= settings.radius * settings.radius {
                fragment.phase = FragmentPhase::CollectingImpact {
                    collector,
                    start_tick: current_tick,
                };
                fragment.position = target;
                fragment.movement = Vec2::ZERO;
            }
        }
        FragmentPhase::CollectingImpact { start_tick, .. } => {
            if let Some(target) = target {
                fragment.position = target;
            }
            fragment.movement = Vec2::ZERO;
            if age(start_tick) >= settings.collecting_impact_ticks {
                fragment.phase = FragmentPhase::Gone;
            }
        }
    }
}
