use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lightyear::prelude::*;

use crate::app::ServerState;
use crate::player::PlayerId;
use crate::protocol::inputs::PlayerAction;
use crate::shared::FixedGameplaySet;
use crate::weapon::protocol::{EquippedWeapon, WeaponCooldown, WeaponKind};
use crate::weapon::shared::fire_equipped_weapons;

#[cfg(feature = "server")]
pub struct WeaponServerPlugin;

#[cfg(feature = "server")]
impl Plugin for WeaponServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                ensure_player_weapons,
                switch_player_weapons,
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
        ));
    }
}

fn switch_player_weapons(
    mut players: Query<
        (
            Has<Predicted>,
            &ActionState<PlayerAction>,
            &mut EquippedWeapon,
            &mut WeaponCooldown,
        ),
        With<PlayerId>,
    >,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (predicted, actions, mut equipped, mut cooldown) in &mut players {
        if is_host_server && predicted {
            continue;
        }

        let requested = if actions.just_pressed(&PlayerAction::EquipSword) {
            Some(WeaponKind::Sword)
        } else if actions.just_pressed(&PlayerAction::EquipSpear) {
            Some(WeaponKind::Spear)
        } else if actions.just_pressed(&PlayerAction::EquipStaff) {
            Some(WeaponKind::Staff)
        } else if actions.just_pressed(&PlayerAction::EquipBow) {
            Some(WeaponKind::Bow)
        } else if actions.just_pressed(&PlayerAction::EquipShuriken) {
            Some(WeaponKind::Shuriken)
        } else if actions.just_pressed(&PlayerAction::EquipBoomerang) {
            Some(WeaponKind::Boomerang)
        } else {
            None
        };

        let Some(kind) = requested else {
            continue;
        };

        if equipped.kind == kind {
            continue;
        }

        equipped.kind = kind;
        *cooldown = WeaponCooldown::default();
    }
}
