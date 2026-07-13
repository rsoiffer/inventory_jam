use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentState {
    pub version: u32,
    pub fragment_count: u32,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            version: 1,
            fragment_count: 0,
        }
    }
}

impl PersistentState {
    #[cfg(feature = "server")]
    pub fn hash_hex(&self, username: &str) -> String {
        use sha2::{Digest, Sha256};

        let preimage = format!("v1|{username}|fragment_count={}", self.fragment_count);
        let digest = Sha256::digest(preimage.as_bytes());
        hex::encode(digest)
    }
}

/// Authoritative persistent player state replicated from the server.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Deref, DerefMut)]
pub struct CachedPersistentState(pub PersistentState);
