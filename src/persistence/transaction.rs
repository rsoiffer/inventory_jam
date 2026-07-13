use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::state::PersistentState;

pub trait ApplyPersistenceTransaction {
    fn apply(&self, state: &mut PersistentState) -> Result<(), String>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddFragmentsTransaction {
    pub amount: u32,
}

impl ApplyPersistenceTransaction for AddFragmentsTransaction {
    fn apply(&self, state: &mut PersistentState) -> Result<(), String> {
        if self.amount == 0 {
            return Err("add_fragments amount must be positive".into());
        }
        state.fragment_count = state
            .fragment_count
            .checked_add(self.amount)
            .ok_or_else(|| {
                format!(
                    "fragment_count overflow: {} + {}",
                    state.fragment_count, self.amount
                )
            })?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload")]
pub enum PersistenceTransactionKind {
    #[serde(rename = "add_fragments")]
    AddFragments(AddFragmentsTransaction),
}

impl ApplyPersistenceTransaction for PersistenceTransactionKind {
    fn apply(&self, state: &mut PersistentState) -> Result<(), String> {
        match self {
            Self::AddFragments(transaction) => transaction.apply(state),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistenceTransaction {
    pub id: Uuid,
    pub username: String,
    #[serde(flatten)]
    pub kind: PersistenceTransactionKind,
}

impl PersistenceTransaction {
    pub fn add_fragments(username: impl Into<String>, amount: u32) -> Result<Self, String> {
        if amount == 0 {
            return Err("add_fragments amount must be positive".into());
        }
        Ok(Self {
            id: Uuid::new_v4(),
            username: username.into(),
            kind: PersistenceTransactionKind::AddFragments(AddFragmentsTransaction { amount }),
        })
    }

    pub fn apply_to(&self, state: &mut PersistentState) -> Result<(), String> {
        self.kind.apply(state)
    }
}
