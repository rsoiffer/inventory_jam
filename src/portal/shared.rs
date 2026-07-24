use bevy::math::{Rect, Vec2};
use rand::Rng;

use crate::settings::{PortalSettings, WorldSettings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArenaEdge {
    Left,
    Right,
    Bottom,
    Top,
}

pub fn random_arena_edge_position(
    world: &WorldSettings,
    player_half_size: f32,
    edge_inset: f32,
    rng: &mut impl Rng,
) -> Vec2 {
    let bounds = world.arena_bounds().inflate(-player_half_size);
    let inset = edge_inset.max(0.0);
    let min_x = bounds.min.x + inset;
    let max_x = bounds.max.x - inset;
    let min_y = bounds.min.y + inset;
    let max_y = bounds.max.y - inset;

    let horizontal_span = (max_x - min_x).max(0.0);
    let vertical_span = (max_y - min_y).max(0.0);

    match rng.random_range(0..4) {
        0 => ArenaEdge::Left,
        1 => ArenaEdge::Right,
        2 => ArenaEdge::Bottom,
        _ => ArenaEdge::Top,
    }
    .position(
        bounds,
        min_x,
        max_x,
        min_y,
        max_y,
        horizontal_span,
        vertical_span,
        rng,
    )
}

impl ArenaEdge {
    fn position(
        self,
        bounds: Rect,
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
        horizontal_span: f32,
        vertical_span: f32,
        rng: &mut impl Rng,
    ) -> Vec2 {
        match self {
            Self::Left => Vec2::new(
                bounds.min.x,
                if vertical_span > 0.0 {
                    rng.random_range(min_y..=max_y)
                } else {
                    bounds.center().y
                },
            ),
            Self::Right => Vec2::new(
                bounds.max.x,
                if vertical_span > 0.0 {
                    rng.random_range(min_y..=max_y)
                } else {
                    bounds.center().y
                },
            ),
            Self::Bottom => Vec2::new(
                if horizontal_span > 0.0 {
                    rng.random_range(min_x..=max_x)
                } else {
                    bounds.center().x
                },
                bounds.min.y,
            ),
            Self::Top => Vec2::new(
                if horizontal_span > 0.0 {
                    rng.random_range(min_x..=max_x)
                } else {
                    bounds.center().x
                },
                bounds.max.y,
            ),
        }
    }
}

pub fn place_arena_portals(
    world: &WorldSettings,
    portal: &PortalSettings,
    player_half_size: f32,
    rng: &mut impl Rng,
) -> Vec<Vec2> {
    let bounds = world.arena_bounds().inflate(-player_half_size);
    let mut positions = Vec::with_capacity(portal.arena_portal_count);

    for _ in 0..portal.arena_portal_count {
        let mut best = bounds.center();
        let mut best_clearance = -1.0_f32;

        for _ in 0..portal.placement_attempts {
            let candidate = Vec2::new(
                rng.random_range(bounds.min.x..=bounds.max.x),
                rng.random_range(bounds.min.y..=bounds.max.y),
            );
            let clearance = positions
                .iter()
                .map(|existing: &Vec2| existing.distance_squared(candidate))
                .fold(f32::MAX, f32::min);

            if clearance > best_clearance {
                best = candidate;
                best_clearance = clearance;
            }
        }

        positions.push(best);
    }

    positions
}
