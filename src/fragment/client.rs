use crate::fragment::protocol::{Fragment, FragmentPhase};
use crate::fragment::shared as fragment_shared;
use crate::player::{PlayerId, PlayerPosition};
use crate::protocol::rooms::{GameRoom, GameRooms};
use crate::{app::ClientState, settings::GameSettings};
use bevy::prelude::*;
use lightyear::{core::tick::TickDuration, prediction::Predicted, prelude::LocalTimeline};

pub struct FragmentClientPlugin;

impl Plugin for FragmentClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            simulate_client_fragments.run_if(in_state(ClientState::Playing)),
        );
    }
}

/// Presentation-only: currency is awarded by the server on collection.
fn simulate_client_fragments(
    mut fragments: Query<&mut Fragment>,
    players: Query<(&PlayerId, &PlayerPosition, &GameRoom, Has<Predicted>)>,
    local_timeline: Res<LocalTimeline>,
    tick_duration: Res<TickDuration>,
    settings: Res<GameSettings>,
) {
    let tick = local_timeline.tick();
    let tick_secs = tick_duration.0.as_secs_f32();

    for mut fragment in &mut fragments {
        let target = fragment.phase.collector().and_then(|collector| {
            fragment_shared::collector_position(
                collector,
                players
                    .iter()
                    .filter_map(|(id, position, room, predicted)| {
                        (room.room == GameRooms::Arena).then_some((id.0, position.0, predicted))
                    }),
            )
        });

        if fragment.phase.collector().is_some() && target.is_none() {
            fragment.phase = FragmentPhase::Gone;
            fragment.movement = Vec2::ZERO;
            continue;
        }

        fragment_shared::tick_fragment(&mut fragment, target, tick, tick_secs, &settings.fragment);
    }
}
