use bevy::prelude::*;

use crate::{app::ClientState, shared::WeaponSystemSet, weapon::shared::fire_equipped_weapons};

pub struct WeaponClientPlugin;

impl Plugin for WeaponClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            fire_equipped_weapons
                .in_set(WeaponSystemSet::Fire)
                .run_if(in_state(ClientState::Playing)),
        );
    }
}
