use bevy::prelude::*;
use lightyear::prelude::*;

use crate::app::ServerState;
use crate::player::PlayerId;
use crate::shared::FixedGameplaySet;
use crate::weapon::protocol::{EquippedWeapon, ShotSequence, WeaponCooldown, WeaponKind};
use crate::weapon::shared::{fire_equipped_weapons, switch_equipped_weapons};

#[cfg(feature = "server")]
pub struct WeaponServerPlugin;

#[cfg(feature = "server")]
impl Plugin for WeaponServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                ensure_player_weapons,
                switch_equipped_weapons,
                fire_equipped_weapons,
            )
                .chain()
                .in_set(FixedGameplaySet::Weapon)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

pub(crate) fn ensure_player_weapons(
    mut commands: Commands,
    players: Query<(Entity, Has<Predicted>), (With<PlayerId>, Without<EquippedWeapon>)>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (entity, predicted) in &players {
        if is_host_server && predicted {
            continue;
        }

        commands.entity(entity).insert((
            EquippedWeapon::new(WeaponKind::Sword, 1),
            WeaponCooldown::default(),
            ShotSequence::default(),
        ));
    }
}
