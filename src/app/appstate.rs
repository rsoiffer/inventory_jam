use crate::app::args::*;
use bevy::prelude::{Resource, States};

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    Connecting,
    Playing,
    Hosting,
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

impl LaunchMode {
    pub const fn starts_client(self) -> bool {
        matches!(self, Self::Menu | Self::JoinEdgegap | Self::JoinLocal)
    }

    pub const fn starts_server(self) -> bool {
        matches!(self, Self::DedicatedServer | Self::HostLocal)
            || (matches!(self, Self::Menu) && cfg!(all(feature = "dev", feature = "gui")))
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct LaunchConfig {
    pub username: String,
    pub mode: LaunchMode,
}

pub fn config_from_env() -> LaunchConfig {
    #[cfg(target_family = "wasm")]
    let config = LaunchConfig {
        username: String::new(),
        mode: LaunchMode::Menu,
    };

    #[cfg(not(target_family = "wasm"))]
    let config = parse_args(std::env::args().skip(1)).unwrap_or_else(|error| {
        eprintln!("error: {error}\n{USAGE}");
        std::process::exit(2);
    });

    #[cfg(all(not(feature = "gui"), feature = "server", not(feature = "client")))]
    let config = if config.mode == LaunchMode::Menu {
        LaunchConfig {
            mode: LaunchMode::DedicatedServer,
            ..config
        }
    } else {
        config
    };

    #[cfg(all(not(feature = "gui"), feature = "client"))]
    if config.mode == LaunchMode::Menu {
        eprintln!("headless builds require --mode and client modes require --user\n{USAGE}");
        std::process::exit(2);
    }

    config
}
