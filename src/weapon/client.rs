//! Client weapon systems run shared predicted fire and equip logic.

use bevy::prelude::*;

use crate::{
    app::ClientState,
    shared::FixedGameplaySet,
    weapon::shared::{fire_equipped_weapons, switch_equipped_weapons},
};

pub struct WeaponClientPlugin;

impl Plugin for WeaponClientPlugin {
    fn build(&self, app: &mut App) {
        // Listen-host runs the server weapon systems only; pure clients run these.
        #[cfg(feature = "server")]
        {
            use crate::app::ServerState;

            app.add_systems(
                FixedUpdate,
                (switch_equipped_weapons, fire_equipped_weapons)
                    .chain()
                    .in_set(FixedGameplaySet::Weapon)
                    .run_if(in_state(ClientState::Playing))
                    .run_if(not(in_state(ServerState::Hosting))),
            );
        }

        #[cfg(not(feature = "server"))]
        {
            app.add_systems(
                FixedUpdate,
                (switch_equipped_weapons, fire_equipped_weapons)
                    .chain()
                    .in_set(FixedGameplaySet::Weapon)
                    .run_if(in_state(ClientState::Playing)),
            );
        }
    }
}
