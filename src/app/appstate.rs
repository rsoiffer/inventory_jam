use super::args::parse_args;
use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    Connecting,
    Playing,
    Hosting,
}

pub(crate) fn game_is_active(state: Res<State<AppState>>) -> bool {
    matches!(state.get(), AppState::Playing | AppState::Hosting)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LaunchMode {
    #[default]
    Menu,
    JoinEdgegap,
    JoinLocal,
    HostLocal,
    DedicatedServer,
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct LaunchConfig {
    pub username: String,
    pub mode: LaunchMode,
}

#[derive(Message)]
pub struct StartGame {
    pub mode: LaunchMode,
    pub username: String,
}

pub struct LaunchPlugin;

impl Plugin for LaunchPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<StartGame>()
            .add_systems(Startup, start_configured_game);
    }
}

fn start_configured_game(config: Res<LaunchConfig>, mut starts: MessageWriter<StartGame>) {
    if config.mode != LaunchMode::Menu {
        starts.write(StartGame {
            mode: config.mode,
            username: config.username.clone(),
        });
    }
}

pub fn config_from_env() -> LaunchConfig {
    #[cfg(target_family = "wasm")]
    let config = LaunchConfig {
        username: String::new(),
        mode: LaunchMode::Menu,
    };

    #[cfg(not(target_family = "wasm"))]
    let config = parse_args();

    #[cfg(all(not(feature = "gui"), feature = "server", not(feature = "client")))]
    let config = if config.mode == LaunchMode::Menu {
        LaunchConfig {
            mode: LaunchMode::DedicatedServer,
            ..config
        }
    } else {
        config
    };

    config
}
