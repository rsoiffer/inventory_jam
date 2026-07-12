use bevy::prelude::Resource;
#[cfg(feature = "client")]
use std::hash::{DefaultHasher, Hash, Hasher};

pub const MAX_USERNAME_LEN: usize = 16;

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
