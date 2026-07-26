use bevy::prelude::*;

use crate::app::ServerState;
use crate::player::PlayerId;
use crate::shared::WeaponSystemSet;
use crate::weapon::protocol::WeaponCooldown;
use crate::weapon::shared::fire_equipped_weapons;

#[cfg(feature = "server")]
pub struct WeaponServerPlugin;

#[cfg(feature = "server")]
impl Plugin for WeaponServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                ensure_player_weapon_cooldowns.in_set(WeaponSystemSet::Stations),
                fire_equipped_weapons.in_set(WeaponSystemSet::Fire),
            )
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

pub(crate) fn ensure_player_weapon_cooldowns(
    mut commands: Commands,
    players: Query<Entity, (With<PlayerId>, Without<WeaponCooldown>)>,
) {
    for entity in &players {
        commands.entity(entity).insert(WeaponCooldown::default());
    }
}
