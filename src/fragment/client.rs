use crate::{app::ClientState, settings::GameSettings};
use bevy::prelude::*;
use lightyear::{prediction::Predicted, prelude::PeerId};

/// Installs only client-side fragment presentation behavior.
pub struct FragmentClientPlugin;

impl Plugin for FragmentClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(client::initialize_fragment);
        app.add_systems(
            FixedUpdate,
            client::simulate_client_fragments.run_if(in_state(ClientState::Playing)),
        );
    }
}

use crate::fragment::{
    client,
    protocol::Fragment,
    shared::{self as fragment_shared, ClientFragment, FragmentPosition, ServerFragment},
};
use crate::player::{PlayerId, PlayerPosition};

/// Creates client-only presentation state when a replicated fragment arrives.
pub(crate) fn initialize_fragment(
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
pub(crate) fn simulate_client_fragments(
    mut fragments: Query<
        (&Fragment, &mut FragmentPosition),
        (With<ClientFragment>, Without<ServerFragment>),
    >,
    players: Query<(&PlayerId, &PlayerPosition, Has<Predicted>)>,
    settings: Res<GameSettings>,
) {
    for (fragment, mut position) in &mut fragments {
        let Some(collector) = fragment.collector else {
            continue;
        };

        let Some(target) = collector_position(collector, &players) else {
            continue;
        };

        fragment_shared::pull_fragment_toward(&mut position, target, &settings.fragment);
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
