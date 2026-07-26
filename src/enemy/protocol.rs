use bevy::math::Curve;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::settings::GameSettings;

pub struct EnemyProtocolPlugin;

impl Plugin for EnemyProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.component::<EnemyPosition>()
            .replicate()
            .predict()
            .add_linear_interpolation();
        app.component::<EnemyHealth>().replicate().predict();
        app.component::<EnemyIdentity>().replicate().predict();
        app.component::<EnemyAi>().replicate().predict();
        app.component::<EnemySpawnerPosition>().replicate();
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct EnemyPosition(pub Vec2);

impl Ease for EnemyPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            EnemyPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyHealth {
    pub current: u32,
    pub maximum: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Regular,
    GrandChampion,
}

impl EnemyKind {
    pub fn scale(self, base: f32) -> f32 {
        match self {
            Self::Regular => base,
            Self::GrandChampion => base * 4.0,
        }
    }
}

/// Replicated identity; combat numbers come from settings + this.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyIdentity {
    pub kind: EnemyKind,
    pub tier_index: u8,
    pub is_ranged: bool,
    pub ai_seed: u64,
}

impl EnemyIdentity {
    pub fn stats(self, settings: &GameSettings) -> EnemyStats {
        match self.kind {
            EnemyKind::Regular => {
                let tier = settings.spawner.tier(self.tier_index);
                EnemyStats {
                    contact_damage: if self.is_ranged {
                        0
                    } else {
                        tier.contact_damage
                    },
                    ranged_damage: if self.is_ranged {
                        tier.ranged_damage
                    } else {
                        0
                    },
                    attacks_per_second: settings.enemy.attacks_per_second,
                    fragment_multiplier: tier.fragment_multiplier,
                    move_speed: tier.move_speed,
                    personal_aristeia_reward: tier.personal_aristeia_reward,
                    global_aristeia_reward: tier.global_aristeia_reward,
                }
            }
            EnemyKind::GrandChampion => EnemyStats {
                contact_damage: settings.global_aristeia.grand_champion_contact_damage,
                ranged_damage: settings.global_aristeia.grand_champion_ranged_damage,
                attacks_per_second: 1.0,
                fragment_multiplier: settings.global_aristeia.grand_champion_fragment_multiplier,
                move_speed: settings.global_aristeia.grand_champion_move_speed,
                personal_aristeia_reward: settings
                    .global_aristeia
                    .grand_champion_personal_aristeia_reward,
                global_aristeia_reward: 0,
            },
        }
    }

    pub fn max_health(self, settings: &GameSettings) -> u32 {
        match self.kind {
            EnemyKind::Regular => settings.spawner.tier(self.tier_index).health,
            EnemyKind::GrandChampion => settings.global_aristeia.grand_champion_health,
        }
    }

    pub fn projectile_buffer_capacity(self, settings: &GameSettings) -> usize {
        match self.kind {
            EnemyKind::Regular => settings.projectile.buffer_capacity,
            EnemyKind::GrandChampion => {
                settings
                    .global_aristeia
                    .grand_champion_projectile_buffer_capacity
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyStats {
    pub contact_damage: u32,
    pub ranged_damage: u32,
    pub attacks_per_second: f32,
    pub fragment_multiplier: f32,
    pub move_speed: f32,
    pub personal_aristeia_reward: u32,
    pub global_aristeia_reward: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EnemyBehavior {
    #[default]
    Wandering,
    Chasing(PeerId),
    Returning,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossPatternKind {
    Fan,
    Radial,
    Spiral,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct BossAttackState {
    pub pattern: BossPatternKind,
    pub pattern_elapsed_seconds: f32,
    pub volley_accumulator: f32,
    pub spiral_angle_radians: f32,
    pub radial_offset: bool,
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct EnemyAi {
    pub home: Vec2,
    pub wander_target: Vec2,
    pub behavior: EnemyBehavior,
    pub engage_retarget_seconds: f32,
    pub boss: Option<BossAttackState>,
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Deref, DerefMut)]
pub struct EnemySpawnerPosition(pub Vec2);
