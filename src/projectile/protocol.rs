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
        weapon: WeaponId,
        position: Vec2,
        velocity: Vec2,
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
                weapon,
                position: end_position,
                velocity,
                state,
            },
        ) if start_generation == end_generation => ProjectileSlot::Active {
            generation: end_generation,
            weapon,
            position: Vec2::lerp(start_position, end_position, t),
            velocity,
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

    pub fn insert(&mut self, weapon: WeaponId, position: Vec2, velocity: Vec2, spawn_tick: Tick) {
        let index = (self.next_index as usize) % self.slots.len();
        let generation = match self.slots[index] {
            ProjectileSlot::Empty { generation } | ProjectileSlot::Active { generation, .. } => {
                generation.wrapping_add(1)
            }
        };
        self.slots[index] = ProjectileSlot::Active {
            generation,
            weapon,
            position,
            velocity,
            state: ProjectileState::Flying { spawn_tick },
        };
        self.next_index = self.next_index.wrapping_add(1);
    }

    /// Advance slots one fixed tick. `on_hit` returns true to convert flight into impact.
    pub fn simulate(
        &mut self,
        room: &GameRoom,
        tick: Tick,
        tick_secs: f32,
        settings: &GameSettings,
        mut on_hit: impl FnMut(WeaponId, Vec2) -> bool,
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
                    weapon,
                    position,
                    velocity,
                    state: ProjectileState::Flying { spawn_tick },
                } => {
                    *position += *velocity * tick_secs;
                    let generation = *generation;
                    let weapon = *weapon;
                    let position = *position;
                    let velocity = *velocity;
                    let spawn_tick = *spawn_tick;

                    let Some(stats) = settings.weapons.get(weapon) else {
                        self.slots[i] = ProjectileSlot::Empty { generation };
                        continue;
                    };
                    let speed_per_tick = velocity.length() * tick_secs.max(0.001);
                    let expired = tick
                        > spawn_tick
                            + Tick((stats.range / speed_per_tick.max(0.001)).ceil() as u32)
                        || !room.bounds(&settings.world).contains(position);

                    self.slots[i] = if on_hit(weapon, position) {
                        ProjectileSlot::Active {
                            generation,
                            weapon,
                            position,
                            velocity,
                            state: ProjectileState::Impact {
                                remaining_ticks: settings.projectile.impact_lifetime_ticks,
                            },
                        }
                    } else if expired {
                        ProjectileSlot::Empty { generation }
                    } else {
                        ProjectileSlot::Active {
                            generation,
                            weapon,
                            position,
                            velocity,
                            state: ProjectileState::Flying { spawn_tick },
                        }
                    };
                }
            }
        }
    }
}
