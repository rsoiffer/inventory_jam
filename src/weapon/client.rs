//! Client weapon behavior is intentionally presentation-only.
//!
//! The replicated `EquippedWeapon` component is rendered by `weapon::render`.
//! Damage, cooldowns, firing, and hit resolution remain server-authoritative.

use bevy::app::{App, Plugin};

pub struct WeaponClientPlugin;

impl Plugin for WeaponClientPlugin {
    fn build(&self, _app: &mut App) {}
}
