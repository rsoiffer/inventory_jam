use bevy::prelude::*;

#[cfg(feature = "server")]
use crate::app::AppState;
#[cfg(feature = "server")]
use crate::shared::FixedGameplaySet;

#[cfg(feature = "client")]
pub(crate) mod client;

pub mod protocol;

#[cfg(feature = "gui")]
pub(crate) mod render;

#[cfg(feature = "server")]
pub(crate) mod server;

pub mod shared;

/// Installs weapon component registration on every peer before connections are
/// spawned.
pub struct WeaponProtocolPlugin;

impl Plugin for WeaponProtocolPlugin {
    fn build(&self, app: &mut App) {
        protocol::register(app);
    }
}

/// Installs server-authoritative weapon state and firing behavior.
#[cfg(feature = "server")]
pub struct WeaponServerPlugin;

#[cfg(feature = "server")]
impl Plugin for WeaponServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                server::ensure_player_weapons,
                server::ensure_weapon_cooldowns,
                server::tick_weapon_cooldowns,
                server::fire_equipped_weapons,
            )
                .chain()
                .in_set(FixedGameplaySet::WeaponSimulation)
                .run_if(in_state(AppState::Hosting)),
        );
    }
}

/// Installs client-only held-weapon rendering.
#[cfg(feature = "gui")]
pub struct WeaponRenderPlugin;

#[cfg(feature = "gui")]
impl Plugin for WeaponRenderPlugin {
    fn build(&self, app: &mut App) {
        render::register(app);
    }
}
