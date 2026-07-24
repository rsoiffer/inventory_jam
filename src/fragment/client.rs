use crate::fragment::{
    protocol::Fragment,
    shared::{
        self as fragment_shared, ClientFragment, FragmentMagnetAge, FragmentPosition,
        ServerFragment,
    },
};
use crate::player::{PlayerId, PlayerPosition};
use crate::{app::ClientState, settings::GameSettings};
use bevy::prelude::*;
use lightyear::{core::tick::TickDuration, prediction::Predicted, prelude::PeerId};

/// Installs only client-side fragment presentation behavior.
pub struct FragmentClientPlugin;

impl Plugin for FragmentClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(initialize_fragment);
        app.add_systems(
            FixedUpdate,
            simulate_client_fragments.run_if(in_state(ClientState::Playing)),
        );
    }
}

/// Creates client-only presentation state when a replicated fragment arrives.
fn initialize_fragment(
    trigger: On<Add, Fragment>,
    mut commands: Commands,
    fragments: Query<(&Fragment, Option<&ServerFragment>)>,
) {
    let entity = trigger.entity;

    let Ok((fragment, server_fragment)) = fragments.get(entity) else {
        return;
    };

    if server_fragment.is_some() {
        return;
    }

    let fragment_owner_string = if let Some(f) = fragment.collector {
        f.to_string()
    } else {
        "?".to_string()
    };

    commands.entity(entity).insert((
        Name::new(format!("Fragment: {}", fragment_owner_string)),
        FragmentPosition(fragment.origin),
        ClientFragment,
    ));
}

/// Animates collected fragments locally. The server already awarded the
/// currency; this system is presentation only.
fn simulate_client_fragments(
    mut commands: Commands,
    mut fragments: Query<
        (
            Entity,
            &Fragment,
            &mut FragmentPosition,
            Option<&FragmentMagnetAge>,
        ),
        (With<ClientFragment>, Without<ServerFragment>),
    >,
    players: Query<(&PlayerId, &PlayerPosition, Has<Predicted>)>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let tick_secs = tick_duration.0.as_secs_f32();

    for (entity, fragment, mut position, magnet_age) in &mut fragments {
        let Some(collector) = fragment.collector else {
            continue;
        };

        let Some(target) = collector_position(collector, &players) else {
            continue;
        };

        let mut age = magnet_age.copied().unwrap_or_default();
        fragment_shared::pull_fragment_toward(
            &mut position,
            target,
            age.0,
            tick_secs,
            &settings.fragment,
        );
        age.0 = age.0.saturating_add(1);
        commands.entity(entity).insert(age);
    }
}

/// Prefer the locally predicted representation when the collector is the local
/// player. Fall back to any matching replicated representation otherwise.
fn collector_position(
    collector: PeerId,
    players: &Query<(&PlayerId, &PlayerPosition, Has<Predicted>)>,
) -> Option<Vec2> {
    let mut fallback = None;

    for (player_id, position, predicted) in players.iter() {
        if player_id.0 != collector {
            continue;
        }

        if predicted {
            return Some(position.0);
        }

        fallback = Some(position.0);
    }

    fallback
}
