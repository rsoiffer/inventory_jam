use crate::settings::GameSettings;

pub fn roll_weapon_damage(
    settings: &GameSettings,
    base_damage: u32,
    damage_level: u32,
    crit_chance: f32,
    generation: u32,
    pierce_remaining: u16,
) -> (u32, bool) {
    let mut damage = settings.progression.damage_at(base_damage, damage_level);
    let is_crit = deterministic_unit_float(generation, pierce_remaining) < crit_chance;
    if is_crit {
        damage = (damage as f32 * settings.progression.crit_damage_multiplier).round() as u32;
    }
    (damage, is_crit)
}

fn deterministic_unit_float(generation: u32, pierce_remaining: u16) -> f32 {
    let mut hash = generation
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(u32::from(pierce_remaining).wrapping_mul(0x85EB_CA6B));
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7FEB_352D);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846C_A68B);
    hash ^= hash >> 16;
    (hash as f32) * (1.0 / (u32::MAX as f32))
}
