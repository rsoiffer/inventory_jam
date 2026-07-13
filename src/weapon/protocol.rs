use crate::weapon::protocol;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

/// Installs weapon component registration on every peer before connections are
/// spawned.
pub struct WeaponProtocolPlugin;

impl Plugin for WeaponProtocolPlugin {
    fn build(&self, app: &mut App) {
        protocol::register(app);
    }
}

/// Stable gameplay identifier replicated over the network.
///
/// Clients map this ID to local art and audio. Asset paths are deliberately not
/// replicated because they are presentation data, not authoritative state.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeaponKind {
    Sword,
    Spear,
    Staff,
}

/// The weapon currently equipped by a player.
///
/// The server owns this component and replicates it to every client. Weapon
/// stats are resolved from `WeaponKind` and `level` on the server.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EquippedWeapon {
    pub kind: WeaponKind,
    pub level: u32,
}

impl EquippedWeapon {
    pub const fn new(kind: WeaponKind, level: u32) -> Self {
        Self { kind, level }
    }
}

pub fn register(app: &mut App) {
    app.component::<EquippedWeapon>().replicate();
}
