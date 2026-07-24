//! Client weapon behavior is intentionally presentation-only.
//!
//! The replicated `EquippedWeapon` component is rendered by `weapon::render`.
//! Damage, cooldowns, firing, and hit resolution remain server-authoritative.

use bevy::{
    app::{App, FixedUpdate, Plugin},
    ecs::schedule::IntoScheduleConfigs,
    state::condition::in_state,
};

use crate::{app::ClientState, shared::FixedGameplaySet, weapon::shared::fire_equipped_weapons};

pub struct WeaponClientPlugin;

impl Plugin for WeaponClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (fire_equipped_weapons)
                .chain()
                .in_set(FixedGameplaySet::Weapon)
                .run_if(in_state(ClientState::Playing)),
        );
    }
}
