use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{
    app::ServerState,
    gate::{
        GateKind, GateOpen, GatePosition, GateProgress,
        shared::{place_arena_gates, random_arena_edge_position},
    },
    player::protocol::{PlayerHealth, PlayerId, PlayerPosition},
    protocol::rooms::{GameRoom, GameRooms},
    settings::GameSettings,
    shared::FixedGameplaySet,
};

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct GateTeleportCooldown {
    pub remaining_ticks: u16,
}

pub struct GateServerPlugin;

impl Plugin for GateServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ServerState::Hosting), spawn_gates);
        app.add_systems(
            FixedUpdate,
            (
                tick_gate_teleport_cooldowns,
                tick_arena_gates,
                apply_gate_touch_teleports,
            )
                .chain()
                .in_set(FixedGameplaySet::Player)
                .run_if(in_state(ServerState::Hosting)),
        );
    }
}

fn spawn_gates(
    mut commands: Commands,
    settings: Res<GameSettings>,
    existing: Query<(), With<GateKind>>,
) {
    if !existing.is_empty() {
        return;
    }

    let mut rng = rand::rng();
    let safezone_gate_position =
        settings.world.safezone_bounds().center() + settings.gate.safezone_gate_offset;

    commands.spawn((
        Name::new("Safezone Gate"),
        GateKind::ToArena,
        GatePosition(safezone_gate_position),
        GateOpen(true),
        GateProgress(1.0),
        Replicate::to_clients(NetworkTarget::All),
    ));

    for position in place_arena_gates(
        &settings.world,
        &settings.gate,
        settings.player.half_size,
        &mut rng,
    ) {
        commands.spawn((
            Name::new("Arena Gate"),
            GateKind::ToSafezone,
            GatePosition(position),
            GateOpen(false),
            GateProgress(0.0),
            Replicate::to_clients(NetworkTarget::All),
        ));
    }
}

fn tick_gate_teleport_cooldowns(mut players: Query<&mut GateTeleportCooldown, With<PlayerId>>) {
    for mut cooldown in &mut players {
        cooldown.remaining_ticks = cooldown.remaining_ticks.saturating_sub(1);
    }
}

fn tick_arena_gates(
    time: Res<Time>,
    settings: Res<GameSettings>,
    players: Query<(&PlayerPosition, &PlayerHealth, &GameRoom)>,
    mut gates: Query<(&GateKind, &GatePosition, &mut GateOpen, &mut GateProgress)>,
) {
    let nearby_radius = settings.gate.nearby_radius + settings.player.collision_radius;
    let nearby_radius_squared = nearby_radius * nearby_radius;
    let dt = time.delta_secs();
    let fill_rate = 1.0 / settings.gate.open_fill_seconds.max(f32::EPSILON);
    let closed_decay_rate = 1.0 / settings.gate.closed_decay_seconds.max(f32::EPSILON);
    let open_drain_rate = 1.0 / settings.gate.open_drain_seconds.max(f32::EPSILON);

    let nearby_players: Vec<Vec2> = players
        .iter()
        .filter(|(_, health, room)| health.current > 0 && room.room == GameRooms::Arena)
        .map(|(position, _, _)| position.0)
        .collect();

    for (kind, position, mut open, mut progress) in &mut gates {
        if *kind != GateKind::ToSafezone {
            continue;
        }

        let player_nearby = nearby_players
            .iter()
            .any(|player| player.distance_squared(position.0) <= nearby_radius_squared);

        if open.0 {
            progress.0 = (progress.0 - open_drain_rate * dt).max(0.0);
            open.0 = progress.0 > 0.0;
        } else if player_nearby {
            progress.0 = (progress.0 + fill_rate * dt).min(1.0);
            open.0 = progress.0 >= 1.0;
        } else {
            progress.0 = (progress.0 - closed_decay_rate * dt).max(0.0);
        }
    }
}

fn apply_gate_touch_teleports(
    settings: Res<GameSettings>,
    mut players: Query<
        (
            &mut PlayerPosition,
            &mut GameRoom,
            &mut GateTeleportCooldown,
            &PlayerHealth,
        ),
        With<ControlledBy>,
    >,
    gates: Query<(&GateKind, &GatePosition, &GateOpen)>,
) {
    let teleport_radius = settings.gate.teleport_radius + settings.player.collision_radius;
    let teleport_radius_squared = teleport_radius * teleport_radius;
    let safezone_center = settings.world.safezone_bounds().center();
    let mut rng = rand::rng();

    for (mut position, mut room, mut cooldown, health) in &mut players {
        if health.current == 0 || cooldown.remaining_ticks > 0 {
            continue;
        }

        let Some((kind, _, _)) = gates.iter().find(|(kind, gate_position, open)| {
            kind.source_room() == room.room
                && (matches!(kind, GateKind::ToArena) || open.0)
                && position.0.distance_squared(gate_position.0) <= teleport_radius_squared
        }) else {
            continue;
        };

        room.room = kind.destination_room();
        position.0 = match kind {
            GateKind::ToArena => random_arena_edge_position(
                &settings.world,
                settings.player.half_size,
                settings.gate.arena_edge_inset,
                &mut rng,
            ),
            GateKind::ToSafezone => safezone_center,
        };
        cooldown.remaining_ticks = settings.gate.teleport_cooldown_ticks;
    }
}
