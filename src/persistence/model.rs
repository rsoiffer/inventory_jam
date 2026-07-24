use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "server")]
use uuid::Uuid;

use crate::weapon::protocol::WeaponId;

#[cfg(feature = "server")]
#[derive(Component, Debug, Default)]
pub struct PersistenceReady;

#[cfg(feature = "server")]
#[derive(Component, Debug, Default)]
pub struct PersistenceLoading;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponProgress {
    pub fragments: u32,
    pub level: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentState {
    pub version: u32,
    pub weapons: BTreeMap<String, WeaponProgress>,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            version: 2,
            weapons: BTreeMap::new(),
        }
    }
}

impl PersistentState {
    pub fn weapon(&self, weapon_id: WeaponId) -> WeaponProgress {
        self.weapons
            .get(&weapon_id.to_string())
            .cloned()
            .unwrap_or_default()
    }

    pub fn weapon_mut(&mut self, weapon_id: WeaponId) -> &mut WeaponProgress {
        self.weapons.entry(weapon_id.to_string()).or_default()
    }

    #[cfg(feature = "server")]
    pub fn hash_hex(&self, username: &str) -> String {
        use sha2::{Digest, Sha256};

        let weapons = self
            .weapons
            .iter()
            .map(|(id, progress)| {
                format!(
                    "{id}:{{fragments={},level={}}}",
                    progress.fragments, progress.level
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let preimage = format!("v2|{username}|weapons=[{weapons}]");
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
    pub weapon_id: WeaponId,
    pub fragment_delta: i64,
    pub level_delta: i64,
}

#[cfg(feature = "server")]
impl PlayerChange {
    pub fn apply(&self, state: &mut PersistentState) -> Result<(), String> {
        if self.fragment_delta == 0 && self.level_delta == 0 {
            return Err("fragment_delta and level_delta cannot both be zero".into());
        }

        let progress = state.weapon_mut(self.weapon_id);
        let next_fragments = (progress.fragments as i64)
            .checked_add(self.fragment_delta)
            .ok_or_else(|| {
                format!(
                    "fragment overflow: {} + {}",
                    progress.fragments, self.fragment_delta
                )
            })?;
        if next_fragments < 0 {
            return Err(format!(
                "fragment balance cannot go negative: {} + {}",
                progress.fragments, self.fragment_delta
            ));
        }
        if next_fragments > u32::MAX as i64 {
            return Err(format!("fragment balance exceeds u32: {next_fragments}"));
        }

        let next_level = (progress.level as i64)
            .checked_add(self.level_delta)
            .ok_or_else(|| format!("level overflow: {} + {}", progress.level, self.level_delta))?;
        if next_level < 0 {
            return Err(format!(
                "level cannot go negative: {} + {}",
                progress.level, self.level_delta
            ));
        }
        if next_level > u32::MAX as i64 {
            return Err(format!("level exceeds u32: {next_level}"));
        }

        progress.fragments = next_fragments as u32;
        progress.level = next_level as u32;
        Ok(())
    }
}

#[cfg(feature = "server")]
impl Transaction {
    pub fn add_weapon_fragments(
        username: impl Into<String>,
        weapon_id: WeaponId,
        amount: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            changes: vec![PlayerChange {
                username: username.into(),
                weapon_id,
                fragment_delta: amount as i64,
                level_delta: 0,
            }],
        }
    }

    pub fn upgrade_weapon(username: impl Into<String>, weapon_id: WeaponId, cost: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            changes: vec![PlayerChange {
                username: username.into(),
                weapon_id,
                fragment_delta: -(cost as i64),
                level_delta: 1,
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
