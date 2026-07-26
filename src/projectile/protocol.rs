use crate::weapon::protocol::WeaponId;
use bevy::math::Curve;
use bevy::prelude::{App, Component, Ease, FunctionCurve, Interval, Plugin, Vec2};
use lightyear::prelude::{
    AppComponentExt, InterpolationRegistrationExt, PredictionBuilderExt, Tick,
};
use serde::{Deserialize, Serialize};

use crate::{protocol::rooms::GameRoom, settings::GameSettings};

pub struct ProjectileProtocolPlugin;

impl Plugin for ProjectileProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<ProjectileBuffer>()
            .replicate()
            .predict()
            .add_linear_interpolation();
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileSource {
    Weapon(WeaponId),
    Enemy,
    GrandChampion,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum ProjectileState {
    Flying { spawn_tick: Tick },
    Impact { remaining_ticks: u16 },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum ProjectileSlot {
    Empty {
        generation: u32,
    },
    Active {
        generation: u32,
        source: ProjectileSource,
        position: Vec2,
        velocity: Vec2,
        pierce_remaining: u16,
        last_hit: u64,
        state: ProjectileState,
    },
}

pub fn overlaps(
    projectile_position: Vec2,
    target_position: Vec2,
    projectile_radius: f32,
    target_radius: f32,
) -> bool {
    let radius = projectile_radius + target_radius;
    projectile_position.distance_squared(target_position) <= radius * radius
}

pub fn projectile_stats(
    source: ProjectileSource,
    settings: &GameSettings,
) -> Option<(f32, f32, f32)> {
    match source {
        ProjectileSource::Weapon(weapon) => settings
            .weapons
            .get(weapon)
            .map(|stats| (stats.range, stats.projectile_radius, stats.projectile_speed)),
        ProjectileSource::Enemy => Some((
            settings.enemy.projectile_range,
            settings.enemy.projectile_radius,
            settings.enemy.projectile_speed,
        )),
        ProjectileSource::GrandChampion => Some((
            settings.global_aristeia.grand_champion_projectile_range,
            settings.global_aristeia.grand_champion_projectile_radius,
            settings.global_aristeia.grand_champion_projectile_speed,
        )),
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProjectileBuffer {
    pub slots: Vec<ProjectileSlot>,
    pub next_index: u16,
}

impl Ease for ProjectileBuffer {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            let slot_count = end.slots.len();
            let mut slots = Vec::with_capacity(slot_count);
            for i in 0..slot_count {
                let start_slot = start
                    .slots
                    .get(i)
                    .copied()
                    .unwrap_or(ProjectileSlot::Empty { generation: 0 });
                let end_slot = end.slots[i];
                slots.push(interpolate_slot(start_slot, end_slot, t));
            }

            ProjectileBuffer {
                slots,
                next_index: end.next_index,
            }
        })
    }
}

fn interpolate_slot(start: ProjectileSlot, end: ProjectileSlot, t: f32) -> ProjectileSlot {
    match (start, end) {
        (
            ProjectileSlot::Active {
                generation: start_generation,
                position: start_position,
                ..
            },
            ProjectileSlot::Active {
                generation: end_generation,
                source,
                position: end_position,
                velocity,
                pierce_remaining,
                last_hit,
                state,
            },
        ) if start_generation == end_generation => ProjectileSlot::Active {
            generation: end_generation,
            source,
            position: Vec2::lerp(start_position, end_position, t),
            velocity,
            pierce_remaining,
            last_hit,
            state,
        },
        (_, end) => end,
    }
}

impl ProjectileBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: vec![ProjectileSlot::Empty { generation: 0 }; capacity],
            next_index: 0,
        }
    }

    pub fn insert(
        &mut self,
        source: ProjectileSource,
        position: Vec2,
        velocity: Vec2,
        pierce_remaining: u16,
        spawn_tick: Tick,
    ) {
        let index = (self.next_index as usize) % self.slots.len();
        let generation = match self.slots[index] {
            ProjectileSlot::Empty { generation } | ProjectileSlot::Active { generation, .. } => {
                generation.wrapping_add(1)
            }
        };
        self.slots[index] = ProjectileSlot::Active {
            generation,
            source,
            position,
            velocity,
            pierce_remaining,
            last_hit: 0,
            state: ProjectileState::Flying { spawn_tick },
        };
        self.next_index = self.next_index.wrapping_add(1);
    }

    /// Advance slots one fixed tick.
    /// `on_hit` returns the hit entity bits when a new target is struck.
    pub fn simulate(
        &mut self,
        room: &GameRoom,
        tick: Tick,
        tick_secs: f32,
        settings: &GameSettings,
        mut on_hit: impl FnMut(ProjectileSource, Vec2, u64) -> Option<u64>,
    ) {
        for i in 0..self.slots.len() {
            match &mut self.slots[i] {
                ProjectileSlot::Empty { .. } => {}
                ProjectileSlot::Active {
                    generation,
                    state: ProjectileState::Impact { remaining_ticks },
                    ..
                } => {
                    *remaining_ticks = remaining_ticks.saturating_sub(1);
                    if *remaining_ticks == 0 {
                        let generation = *generation;
                        self.slots[i] = ProjectileSlot::Empty { generation };
                    }
                }
                ProjectileSlot::Active {
                    generation,
                    source,
                    position,
                    velocity,
                    pierce_remaining,
                    last_hit,
                    state: ProjectileState::Flying { spawn_tick },
                } => {
                    *position += *velocity * tick_secs;
                    let generation = *generation;
                    let source = *source;
                    let position = *position;
                    let velocity = *velocity;
                    let mut pierce_remaining = *pierce_remaining;
                    let mut last_hit = *last_hit;
                    let spawn_tick = *spawn_tick;

                    let Some((range, _, _)) = projectile_stats(source, settings) else {
                        self.slots[i] = ProjectileSlot::Empty { generation };
                        continue;
                    };
                    let speed_per_tick = velocity.length() * tick_secs.max(0.001);
                    let expired = tick
                        > spawn_tick + Tick((range / speed_per_tick.max(0.001)).ceil() as u32)
                        || !room.bounds(&settings.world).contains(position);

                    let hit = on_hit(source, position, last_hit);
                    self.slots[i] = if let Some(hit_id) = hit {
                        if pierce_remaining > 0 {
                            pierce_remaining -= 1;
                            last_hit = hit_id;
                            ProjectileSlot::Active {
                                generation,
                                source,
                                position,
                                velocity,
                                pierce_remaining,
                                last_hit,
                                state: ProjectileState::Flying { spawn_tick },
                            }
                        } else {
                            ProjectileSlot::Active {
                                generation,
                                source,
                                position,
                                velocity,
                                pierce_remaining: 0,
                                last_hit: hit_id,
                                state: ProjectileState::Impact {
                                    remaining_ticks: settings.projectile.impact_lifetime_ticks,
                                },
                            }
                        }
                    } else if expired {
                        ProjectileSlot::Empty { generation }
                    } else {
                        ProjectileSlot::Active {
                            generation,
                            source,
                            position,
                            velocity,
                            pierce_remaining,
                            last_hit,
                            state: ProjectileState::Flying { spawn_tick },
                        }
                    };
                }
            }
        }
    }
}
