use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
#[derive(Component, Debug, Default)]
pub struct PersistenceReady;

#[cfg(feature = "server")]
#[derive(Component, Debug, Default)]
pub struct PersistenceLoading;

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
        hex::encode(Sha256::digest(preimage.as_bytes()))
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Deref, DerefMut)]
pub struct CachedPersistentState(pub PersistentState);

#[cfg(feature = "server")]
#[derive(Message, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub changes: Vec<PlayerChange>,
}

#[cfg(feature = "server")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerChange {
    pub username: String,
    pub add_fragments: u32,
}

#[cfg(feature = "server")]
impl PlayerChange {
    pub fn apply(&self, state: &mut PersistentState) -> Result<(), String> {
        if self.add_fragments == 0 {
            return Err("add_fragments must be positive".into());
        }
        state.fragment_count = state
            .fragment_count
            .checked_add(self.add_fragments)
            .ok_or_else(|| {
                format!(
                    "fragment_count overflow: {} + {}",
                    state.fragment_count, self.add_fragments
                )
            })?;
        Ok(())
    }
}

#[cfg(feature = "server")]
impl Transaction {
    pub fn add_fragments(username: impl Into<String>, amount: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            changes: vec![PlayerChange {
                username: username.into(),
                add_fragments: amount,
            }],
        }
    }

    pub fn touches_username(&self, username: &str) -> bool {
        self.changes
            .iter()
            .any(|change| change.username == username)
    }

    pub fn apply_for_username(
        &self,
        username: &str,
        state: &mut PersistentState,
    ) -> Result<(), String> {
        for change in self
            .changes
            .iter()
            .filter(|change| change.username == username)
        {
            change.apply(state)?;
        }
        Ok(())
    }
}
