use bevy::prelude::Resource;
#[cfg(feature = "client")]
use std::hash::{DefaultHasher, Hash, Hasher};

pub const MAX_USERNAME_LEN: usize = 16;
#[cfg(feature = "client")]
const MACHINE_ID_PREFIX_LEN: usize = 6;

#[derive(Resource, Debug, Clone)]
pub struct LocalUsername(pub String);

impl LocalUsername {
    pub fn validated(&self) -> Result<String, String> {
        validate_username(&self.0)
    }
}

pub fn validate_username(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("username must not be empty".into());
    }
    if name.chars().count() > MAX_USERNAME_LEN {
        return Err(format!(
            "username must be at most {MAX_USERNAME_LEN} characters"
        ));
    }
    if name.chars().any(char::is_control) {
        return Err("username must not contain control characters".into());
    }
    Ok(name.into())
}

/// Prefixes a username with a stable per-machine id so local-dev identities do
/// not collide across computers sharing the same persistence database.
#[cfg(feature = "client")]
pub fn machine_local_username(raw: &str) -> Result<String, String> {
    let base = validate_username(raw)?;
    let prefix = machine_id_prefix();
    validate_username(&format!("{base}_{prefix}"))
}

#[cfg(feature = "client")]
fn machine_id_prefix() -> String {
    let mut hasher = DefaultHasher::new();
    machine_name().hash(&mut hasher);
    let digest = hasher.finish();
    format!("{digest:016x}")
        .chars()
        .take(MACHINE_ID_PREFIX_LEN)
        .collect()
}

#[cfg(feature = "client")]
fn machine_name() -> String {
    #[cfg(windows)]
    {
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".into())
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOSTNAME")
            .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
            .unwrap_or_else(|_| "unknown".into())
    }
}

#[cfg(feature = "server")]
pub fn temporary_username(peer_bits: u64) -> String {
    format!("Player{}", peer_bits % 10_000)
}

#[cfg(feature = "client")]
pub fn client_id_from_username(username: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    username.hash(&mut hasher);
    hasher.finish().max(1)
}
