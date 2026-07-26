use bevy::prelude::Resource;
use std::hash::{DefaultHasher, Hash, Hasher};

pub const MAX_USERNAME_LEN: usize = 16;

#[derive(Resource, Debug, Clone)]
pub struct LocalUsername(pub String);

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

/// Derives a stable netcode client id from a username.
///
/// Used by both the connecting client (to pick its own id) and the server
/// (to recognize which hardcoded test player a connection belongs to), so
/// the two sides always agree without exchanging any extra messages.
pub fn client_id_from_username(username: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    username.hash(&mut hasher);
    hasher.finish().max(1)
}
