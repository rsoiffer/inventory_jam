use bevy::prelude::*;

use crate::weapon::protocol::WeaponKind;

/// Authoritative combat values resolved from an equipped weapon.
///
/// Keep these values in shared code so server simulation and client-side
/// presentation can agree on geometry. Only the server may use them to decide
/// whether damage is applied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponStats {
    pub damage: u32,
    pub range: f32,
    pub attacks_per_second: f32,
    pub projectile_speed_per_tick: f32,
    pub projectile_radius: f32,
}

/// Server-only firing state attached to an authoritative player entity.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct WeaponCooldown {
    pub remaining_seconds: f32,
}

impl WeaponCooldown {
    pub fn tick(&mut self, delta_seconds: f32) {
        self.remaining_seconds = (self.remaining_seconds - delta_seconds).max(0.0);
    }

    pub fn is_ready(&self) -> bool {
        self.remaining_seconds <= 0.0
    }

    pub fn restart(&mut self, attacks_per_second: f32) {
        self.remaining_seconds = 1.0 / attacks_per_second.max(0.01);
    }
}

/// Returns the final stats for a weapon and level.
///
/// Level scaling is intentionally centralized here. Do not store duplicated
/// damage/range/attack-speed fields on each player.
pub fn weapon_stats(kind: WeaponKind, level: u32) -> WeaponStats {
    let level = level.max(1);
    let bonus_levels = level.saturating_sub(1);

    match kind {
        WeaponKind::Sword => WeaponStats {
            damage: 20 + bonus_levels * 4,
            range: 360.0 + bonus_levels as f32 * 12.0,
            attacks_per_second: 2.5 + bonus_levels as f32 * 0.08,
            projectile_speed_per_tick: 18.0,
            projectile_radius: 18.0,
        },
        WeaponKind::Spear => WeaponStats {
            damage: 32 + bonus_levels * 6,
            range: 520.0 + bonus_levels as f32 * 16.0,
            attacks_per_second: 1.5 + bonus_levels as f32 * 0.05,
            projectile_speed_per_tick: 22.0,
            projectile_radius: 12.0,
        },
        WeaponKind::Staff => WeaponStats {
            damage: 16 + bonus_levels * 3,
            range: 460.0 + bonus_levels as f32 * 14.0,
            attacks_per_second: 3.2 + bonus_levels as f32 * 0.1,
            projectile_speed_per_tick: 16.0,
            projectile_radius: 14.0,
        },
    }
}
