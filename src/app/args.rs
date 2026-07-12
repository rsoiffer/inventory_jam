use super::appstate::{LaunchConfig, LaunchMode};
use super::username::validate_username;

pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<LaunchConfig, String> {
    let mut args = args.into_iter();
    let mut mode = LaunchMode::Menu;
    let mut username = None;

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--mode" => {
                mode = match args.next().as_deref() {
                    Some("server") => LaunchMode::DedicatedServer,
                    Some("host") => LaunchMode::HostLocal,
                    Some("edgegap") => LaunchMode::JoinEdgegap,
                    Some("local") => LaunchMode::JoinLocal,
                    Some(value) => {
                        return Err(format!(
                            "invalid --mode '{value}', expected server|host|edgegap|local"
                        ));
                    }
                    None => {
                        return Err("--mode requires a value: server|host|edgegap|local".into());
                    }
                };
            }
            "--user" => {
                username = Some(args.next().ok_or("--user requires a value")?);
            }
            other if other.starts_with('-') => return Err(format!("unknown flag '{other}'")),
            other => return Err(format!("unexpected argument '{other}'")),
        }
    }

    let username = match (mode, username) {
        (LaunchMode::JoinEdgegap | LaunchMode::JoinLocal, None) => {
            return Err("--user is required for --mode edgegap|local".into());
        }
        (_, Some(name)) => validate_username(&name)?,
        (_, None) => String::new(),
    };

    Ok(LaunchConfig { username, mode })
}

pub const USAGE: &str = "usage:
  cargo run
  cargo run -- --mode server
  cargo run -- --mode host
  cargo run -- --mode edgegap --user Ada
  cargo run -- --mode local --user Ada";
