//! Client weapon systems run shared predicted fire logic.

use bevy::prelude::*;

use crate::{app::ClientState, shared::WeaponSystemSet, weapon::shared::fire_equipped_weapons};

pub struct WeaponClientPlugin;

impl Plugin for WeaponClientPlugin {
    fn build(&self, app: &mut App) {
        // Listen-host runs the server weapon systems only; pure clients run these.
        #[cfg(feature = "server")]
        {
            use crate::app::ServerState;

            app.add_systems(
                FixedUpdate,
                fire_equipped_weapons
                    .in_set(WeaponSystemSet::Fire)
                    .run_if(in_state(ClientState::Playing))
                    .run_if(not(in_state(ServerState::Hosting))),
            );
        }

        #[cfg(not(feature = "server"))]
        {
            app.add_systems(
                FixedUpdate,
                fire_equipped_weapons
                    .in_set(WeaponSystemSet::Fire)
                    .run_if(in_state(ClientState::Playing)),
            );
        }
    }
}
