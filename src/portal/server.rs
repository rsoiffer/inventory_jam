use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    app::ServerState,
    player::protocol::{PlayerHealth, PlayerId, PlayerPosition},
    portal::{
        PortalKind, PortalPosition,
        shared::{place_arena_portals, random_arena_edge_position},
    },
    protocol::rooms::GameRoom,
    settings::GameSettings,
    shared::FixedGameplaySet,
};

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct PortalTeleportCooldown {
    pub remaining_ticks: u16,
}

pub struct PortalServerPlugin;

impl Plugin for PortalServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ServerState::Hosting), spawn_portals);
        app.add_systems(
            FixedUpdate,
            (tick_portal_teleport_cooldowns, apply_portal_touch_teleports)
                .chain()
                .in_set(FixedGameplaySet::Player)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

fn spawn_portals(
    mut commands: Commands,
    settings: Res<GameSettings>,
    existing: Query<(), With<PortalKind>>,
) {
    if !existing.is_empty() {
        return;
    }

    let mut rng = rand::rng();
    let safezone_portal_position =
        settings.world.safezone_bounds().center() + settings.portal.safezone_portal_offset;

    commands.spawn((
        Name::new("Safezone Portal"),
        PortalKind::ToArena,
        PortalPosition(safezone_portal_position),
        Replicate::to_clients(NetworkTarget::All),
    ));

    for position in place_arena_portals(
        &settings.world,
        &settings.portal,
        settings.player.half_size,
        &mut rng,
    ) {
        commands.spawn((
            Name::new("Arena Portal"),
            PortalKind::ToSafezone,
            PortalPosition(position),
            Replicate::to_clients(NetworkTarget::All),
        ));
    }
}

fn tick_portal_teleport_cooldowns(mut players: Query<&mut PortalTeleportCooldown, With<PlayerId>>) {
    for mut cooldown in &mut players {
        cooldown.remaining_ticks = cooldown.remaining_ticks.saturating_sub(1);
    }
}

fn apply_portal_touch_teleports(
    settings: Res<GameSettings>,
    mut players: Query<
        (
            &mut PlayerPosition,
            &mut GameRoom,
            &mut PortalTeleportCooldown,
            &PlayerHealth,
        ),
        With<ControlledBy>,
    >,
    portals: Query<(&PortalKind, &PortalPosition)>,
) {
    let trigger_radius = settings.portal.trigger_radius + settings.player.collision_radius;
    let trigger_radius_squared = trigger_radius * trigger_radius;
    let safezone_center = settings.world.safezone_bounds().center();
    let mut rng = rand::rng();

    for (mut position, mut room, mut cooldown, health) in &mut players {
        if health.current == 0 || cooldown.remaining_ticks > 0 {
            continue;
        }

        let Some((kind, _)) = portals.iter().find(|(kind, portal_position)| {
            kind.source_room() == room.room
                && position.0.distance_squared(portal_position.0) <= trigger_radius_squared
        }) else {
            continue;
        };

        room.room = kind.destination_room();
        position.0 = match kind {
            PortalKind::ToArena => random_arena_edge_position(
                &settings.world,
                settings.player.half_size,
                settings.portal.arena_edge_inset,
                &mut rng,
            ),
            PortalKind::ToSafezone => safezone_center,
        };
        cooldown.remaining_ticks = settings.portal.teleport_cooldown_ticks;
    }
}
