use super::appstate::{LaunchConfig, LaunchMode};
use super::username::validate_username;
use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(about, version)]
struct Args {
    #[arg(
        long,
        value_enum,
        required = cfg!(all(not(feature = "gui"), feature = "client"))
    )]
    mode: Option<Mode>,
    #[arg(
        long,
        value_parser = validate_username,
        required_if_eq_any = [("mode", "edgegap"), ("mode", "local")]
    )]
    user: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Mode {
    Server,
    Host,
    Edgegap,
    Local,
}

pub fn parse_args() -> LaunchConfig {
    let args = Args::parse();
    LaunchConfig {
        username: args.user.unwrap_or_default(),
        mode: match args.mode {
            Some(Mode::Server) => LaunchMode::DedicatedServer,
            Some(Mode::Host) => LaunchMode::HostLocal,
            Some(Mode::Edgegap) => LaunchMode::JoinEdgegap,
            Some(Mode::Local) => LaunchMode::JoinLocal,
            None => LaunchMode::Menu,
        },
    }
}
