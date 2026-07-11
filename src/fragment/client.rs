use bevy::prelude::*;

use crate::fragment::{
    self,
    protocol::Fragment,
    shared::{
        self as projectile_shared, ClientFragmentVisual, FragmentLifetime, FragmentPosition,
        ServerFragment,
    },
};

pub(crate) fn initialize_fragment(
    trigger: On<Add, Fragment>,
    mut commands: Commands,
    fragments: Query<(&Fragment, Option<&ServerFragment>)>,
) {
    let entity = trigger.entity;

    let Ok((fragment, server_fragment)) = fragments.get(entity) else {
        return;
    };

    // In host-client mode, do not let the client visual system claim the server-owned fragment.
    if server_fragment.is_some() {
        return;
    }

    commands.entity(entity).insert((
        FragmentPosition(fragment.origin),
        FragmentLifetime {
            remaining_ticks: fragment::shared::FRAGMENT_LIFETIME_TICKS,
        },
        ClientFragmentVisual,
    ));
}

pub(crate) fn simulate_client_fragments(
    mut commands: Commands,
    mut fragments: Query<
        (
            Entity,
            &Fragment,
            &mut FragmentPosition,
            &mut FragmentLifetime,
        ),
        (With<ClientFragmentVisual>, Without<ServerFragment>),
    >,
) {
    for (entity, fragment, position, mut lifetime) in &mut fragments {
        lifetime.remaining_ticks = lifetime.remaining_ticks.saturating_sub(1);

        if lifetime.remaining_ticks == 0 {
            // Do not despawn the replicated entity locally.
            // The server owns despawn authority.
            commands
                .entity(entity)
                .remove::<(ClientFragmentVisual, FragmentPosition, FragmentLifetime)>();
        }
    }
}
