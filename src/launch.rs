#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;

use core::net::SocketAddr;

#[cfg(not(target_arch = "wasm32"))]
use crate::config::{NetworkSession, default_join_server_addr, default_server_addr};

#[cfg(target_arch = "wasm32")]
use crate::config::{NetworkSession, default_join_server_addr};

#[derive(Clone, Debug)]
pub enum LaunchMode {
    Client {
        session: NetworkSession,
    },
    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    DedicatedServer {
        bind_addr: SocketAddr,
    },
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[arg(long)]
    server: bool,
    #[arg(long, default_value_t = default_server_addr())]
    bind: SocketAddr,
    #[arg(long, default_value_t = default_join_server_addr())]
    server_addr: SocketAddr,
    #[arg(short, long)]
    client_id: Option<u64>,
}

pub fn is_dedicated_server(launch: &LaunchMode) -> bool {
    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    {
        matches!(launch, LaunchMode::DedicatedServer { .. })
    }
    #[cfg(not(all(feature = "server", not(target_arch = "wasm32"))))]
    {
        let _ = launch;
        false
    }
}

pub fn launch_mode() -> LaunchMode {
    #[cfg(target_arch = "wasm32")]
    {
        LaunchMode::Client {
            session: NetworkSession::join(default_join_server_addr(), rand::random()),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let cli = Cli::parse();
        if cli.server {
            LaunchMode::DedicatedServer {
                bind_addr: cli.bind,
            }
        } else {
            LaunchMode::Client {
                session: NetworkSession::join(
                    cli.server_addr,
                    cli.client_id.unwrap_or_else(rand::random),
                ),
            }
        }
    }
}
