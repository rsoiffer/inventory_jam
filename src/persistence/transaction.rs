use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::state::PersistentState;

/// Atomic multi-player persistence transaction.
///
/// Used as a gameplay message, queued for flush, and serialized to Postgres.
#[derive(Message, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub changes: Vec<PlayerChange>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerChange {
    pub username: String,
    pub add_fragments: u32,
}

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

    pub fn usernames(&self) -> impl Iterator<Item = &str> {
        self.changes.iter().map(|change| change.username.as_str())
    }
}
