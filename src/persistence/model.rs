use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "server")]
use uuid::Uuid;

use crate::settings::UpgradeStatKind;
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
    #[serde(default)]
    pub attack_speed_level: u32,
    #[serde(default)]
    pub pierce_level: u32,
    #[serde(default)]
    pub crit_level: u32,
}

impl WeaponProgress {
    pub fn level_for(&self, kind: UpgradeStatKind) -> u32 {
        match kind {
            UpgradeStatKind::Damage => self.level,
            UpgradeStatKind::AttackSpeed => self.attack_speed_level,
            UpgradeStatKind::Pierce => self.pierce_level,
            UpgradeStatKind::Crit => self.crit_level,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentState {
    pub version: u32,
    #[serde(default)]
    pub equipped_weapon_id: WeaponId,
    pub weapons: BTreeMap<String, WeaponProgress>,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            version: 4,
            equipped_weapon_id: 0,
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
                    "{id}:{{fragments={},level={},attack_speed_level={},pierce_level={},crit_level={}}}",
                    progress.fragments,
                    progress.level,
                    progress.attack_speed_level,
                    progress.pierce_level,
                    progress.crit_level
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let preimage = format!(
            "v4|{username}|equipped={}|weapons=[{weapons}]",
            self.equipped_weapon_id
        );
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
    #[serde(default)]
    pub attack_speed_level_delta: i64,
    #[serde(default)]
    pub pierce_level_delta: i64,
    #[serde(default)]
    pub crit_level_delta: i64,
    #[serde(default)]
    pub set_equipped: bool,
}

#[cfg(feature = "server")]
impl PlayerChange {
    fn has_progress_delta(&self) -> bool {
        self.fragment_delta != 0
            || self.level_delta != 0
            || self.attack_speed_level_delta != 0
            || self.pierce_level_delta != 0
            || self.crit_level_delta != 0
    }

    pub fn apply(&self, state: &mut PersistentState) -> Result<(), String> {
        if !self.has_progress_delta() && !self.set_equipped {
            return Err("fragment/stat deltas cannot all be zero unless set_equipped".into());
        }

        if self.has_progress_delta() {
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

            progress.fragments = next_fragments as u32;
            progress.level = apply_level_delta(progress.level, self.level_delta, "level")?;
            progress.attack_speed_level = apply_level_delta(
                progress.attack_speed_level,
                self.attack_speed_level_delta,
                "attack_speed_level",
            )?;
            progress.pierce_level = apply_level_delta(
                progress.pierce_level,
                self.pierce_level_delta,
                "pierce_level",
            )?;
            progress.crit_level =
                apply_level_delta(progress.crit_level, self.crit_level_delta, "crit_level")?;
        }

        if self.set_equipped {
            state.equipped_weapon_id = self.weapon_id;
        }

        Ok(())
    }
}

#[cfg(feature = "server")]
fn apply_level_delta(current: u32, delta: i64, label: &str) -> Result<u32, String> {
    if delta == 0 {
        return Ok(current);
    }
    let next = (current as i64)
        .checked_add(delta)
        .ok_or_else(|| format!("{label} overflow: {current} + {delta}"))?;
    if next < 0 {
        return Err(format!("{label} cannot go negative: {current} + {delta}"));
    }
    if next > u32::MAX as i64 {
        return Err(format!("{label} exceeds u32: {next}"));
    }
    Ok(next as u32)
}

#[cfg(feature = "server")]
impl Transaction {
    fn change(
        username: impl Into<String>,
        weapon_id: WeaponId,
        fragment_delta: i64,
        level_delta: i64,
        attack_speed_level_delta: i64,
        pierce_level_delta: i64,
        crit_level_delta: i64,
        set_equipped: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            changes: vec![PlayerChange {
                username: username.into(),
                weapon_id,
                fragment_delta,
                level_delta,
                attack_speed_level_delta,
                pierce_level_delta,
                crit_level_delta,
                set_equipped,
            }],
        }
    }

    pub fn add_weapon_fragments(
        username: impl Into<String>,
        weapon_id: WeaponId,
        amount: u32,
    ) -> Self {
        Self::change(username, weapon_id, amount as i64, 0, 0, 0, 0, false)
    }

    pub fn upgrade_weapon_stat(
        username: impl Into<String>,
        weapon_id: WeaponId,
        kind: UpgradeStatKind,
        cost: u32,
    ) -> Self {
        let (level, attack_speed, pierce, crit) = match kind {
            UpgradeStatKind::Damage => (1, 0, 0, 0),
            UpgradeStatKind::AttackSpeed => (0, 1, 0, 0),
            UpgradeStatKind::Pierce => (0, 0, 1, 0),
            UpgradeStatKind::Crit => (0, 0, 0, 1),
        };
        Self::change(
            username,
            weapon_id,
            -(cost as i64),
            level,
            attack_speed,
            pierce,
            crit,
            false,
        )
    }

    pub fn equip_weapon(username: impl Into<String>, weapon_id: WeaponId) -> Self {
        Self::change(username, weapon_id, 0, 0, 0, 0, 0, true)
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
