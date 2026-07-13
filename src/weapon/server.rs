use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lightyear::prelude::*;

use crate::projectile::PlayerProjectile;
use crate::protocol::inputs::PlayerAction;
use crate::protocol::player::{PlayerAimDirection, PlayerId, PlayerPosition};
use crate::protocol::rooms::GameRoom;
use crate::{
    projectile::shared::{
        self as projectile_shared, ProjectileLifetime, ProjectilePosition, ServerProjectile,
    },
    weapon::{
        protocol::{EquippedWeapon, WeaponKind},
        shared::{WeaponCooldown, weapon_stats},
    },
};

/// Gives every authoritative player a starter weapon.
///
/// Predicted host-client copies are presentation/simulation mirrors and must
/// not receive an independently authoritative weapon.
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

/// Repairs server-only cooldown state after loadouts change or components are
/// inserted dynamically.
pub(crate) fn ensure_weapon_cooldowns(
    mut commands: Commands,
    players: Query<
        (Entity, Has<Predicted>),
        (
            With<PlayerId>,
            With<EquippedWeapon>,
            Without<WeaponCooldown>,
        ),
    >,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (entity, predicted) in &players {
        if is_host_server && predicted {
            continue;
        }

        commands.entity(entity).insert(WeaponCooldown::default());
    }
}

pub(crate) fn tick_weapon_cooldowns(
    time: Res<Time>,
    mut cooldowns: Query<(&mut WeaponCooldown, Has<Predicted>)>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (mut cooldown, predicted) in &mut cooldowns {
        if is_host_server && predicted {
            continue;
        }

        cooldown.tick(time.delta_secs());
    }
}

/// Converts player fire input into authoritative replicated projectiles.
///
/// Damage, range, speed, and radius are snapshotted into the projectile when it
/// is fired. Changing weapons afterward cannot retroactively alter an existing
/// projectile.
pub(crate) fn fire_equipped_weapons(
    mut commands: Commands,
    mut players: Query<(
        &PlayerId,
        &PlayerPosition,
        &PlayerAimDirection,
        &GameRoom,
        &ActionState<PlayerAction>,
        &EquippedWeapon,
        &mut WeaponCooldown,
        Has<Predicted>,
    )>,
    host_server: Query<(), With<lightyear::connection::host::HostServer>>,
) {
    let is_host_server = !host_server.is_empty();

    for (
        player_id,
        player_position,
        aim_direction,
        room,
        actions,
        equipped_weapon,
        mut cooldown,
        predicted,
    ) in &mut players
    {
        if is_host_server && predicted {
            continue;
        }

        // `pressed` allows attack speed to control automatic repeat while the
        // button is held. Use `just_pressed` here instead for semi-auto weapons.
        if !actions.pressed(&PlayerAction::Fire) || !cooldown.is_ready() {
            continue;
        }

        let direction = aim_direction.0.normalize_or_zero();
        if direction == Vec2::ZERO {
            continue;
        }

        let stats = weapon_stats(equipped_weapon.kind, equipped_weapon.level);
        let spawn_position = projectile_shared::projectile_spawn_position(
            player_position.0,
            direction,
            stats.projectile_radius,
        );

        let projectile = PlayerProjectile {
            owner: player_id.0,
            weapon: equipped_weapon.kind,
            origin: spawn_position,
            direction,
            speed_per_tick: stats.projectile_speed_per_tick,
            damage: stats.damage,
            max_range: stats.range,
            radius: stats.projectile_radius,
        };

        commands.spawn((
            projectile,
            ProjectilePosition(spawn_position),
            ProjectileLifetime::from_projectile(&projectile),
            ServerProjectile,
            *room,
            Replicate::to_clients(NetworkTarget::All),
            Name::new("Server Projectile"),
        ));

        cooldown.restart(stats.attacks_per_second);
    }
}
