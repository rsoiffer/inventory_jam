use crate::{app::ServerState, protocol::ProtocolPlugin};
use bevy::prelude::*;

pub const GAME_NAME: &str = "Arena of Champions";

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixedGameplaySet {
    Player,
    Weapon,
    Projectile,
    Persistence,
}

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                FixedGameplaySet::Player,
                FixedGameplaySet::Weapon,
                FixedGameplaySet::Projectile,
                FixedGameplaySet::Persistence,
            )
                .chain(),
        );

        #[cfg(feature = "server")]
        app.add_systems(FixedUpdate, update_window_title);
    }
}

fn update_window_title(mut window_query: Query<&mut Window>, state: Res<State<ServerState>>) {
    for mut window in window_query.iter_mut() {
        match state.get() {
            ServerState::Stopped => (),
            ServerState::Hosting => window.title = format!("{}: Server", GAME_NAME),
        }
    }
}
