use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::settings::ProgressionSettings;
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
    #[serde(default, alias = "level")]
    pub damage_level: u32,
    #[serde(default)]
    pub attack_speed_level: u32,
    #[serde(default)]
    pub pierce_level: u32,
    #[serde(default)]
    pub crit_level: u32,
    #[serde(default)]
    pub armor_level: u32,
    #[serde(default)]
    pub max_health_level: u32,
}

impl WeaponProgress {
    pub fn level_for(&self, kind: UpgradeStatKind) -> u32 {
        match kind {
            UpgradeStatKind::Damage => self.damage_level,
            UpgradeStatKind::AttackSpeed => self.attack_speed_level,
            UpgradeStatKind::Pierce => self.pierce_level,
            UpgradeStatKind::Crit => self.crit_level,
            UpgradeStatKind::Armor => self.armor_level,
            UpgradeStatKind::MaxHealth => self.max_health_level,
        }
    }

    pub fn set_level_for(&mut self, kind: UpgradeStatKind, level: u32) {
        match kind {
            UpgradeStatKind::Damage => self.damage_level = level,
            UpgradeStatKind::AttackSpeed => self.attack_speed_level = level,
            UpgradeStatKind::Pierce => self.pierce_level = level,
            UpgradeStatKind::Crit => self.crit_level = level,
            UpgradeStatKind::Armor => self.armor_level = level,
            UpgradeStatKind::MaxHealth => self.max_health_level = level,
        }
    }

    pub fn total_level(&self) -> u32 {
        self.damage_level
            .saturating_add(self.attack_speed_level)
            .saturating_add(self.pierce_level)
            .saturating_add(self.crit_level)
            .saturating_add(self.armor_level)
            .saturating_add(self.max_health_level)
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
            version: 6,
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
                    "{id}:{{fragments={},damage_level={},attack_speed_level={},pierce_level={},crit_level={},armor_level={},max_health_level={}}}",
                    progress.fragments,
                    progress.damage_level,
                    progress.attack_speed_level,
                    progress.pierce_level,
                    progress.crit_level,
                    progress.armor_level,
                    progress.max_health_level
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let preimage = format!(
            "v6|{username}|equipped={}|weapons=[{weapons}]",
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
    #[serde(default, alias = "level_delta")]
    pub damage_level_delta: i64,
    #[serde(default)]
    pub attack_speed_level_delta: i64,
    #[serde(default)]
    pub pierce_level_delta: i64,
    #[serde(default)]
    pub crit_level_delta: i64,
    #[serde(default)]
    pub armor_level_delta: i64,
    #[serde(default)]
    pub max_health_level_delta: i64,
    #[serde(default)]
    pub set_equipped: bool,
}

#[cfg(feature = "server")]
impl PlayerChange {
    fn has_progress_delta(&self) -> bool {
        self.fragment_delta != 0
            || self.damage_level_delta != 0
            || self.attack_speed_level_delta != 0
            || self.pierce_level_delta != 0
            || self.crit_level_delta != 0
            || self.armor_level_delta != 0
            || self.max_health_level_delta != 0
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
            progress.damage_level = apply_level_delta(
                progress.damage_level,
                self.damage_level_delta,
                "damage_level",
            )?;
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
            progress.armor_level =
                apply_level_delta(progress.armor_level, self.armor_level_delta, "armor_level")?;
            progress.max_health_level = apply_level_delta(
                progress.max_health_level,
                self.max_health_level_delta,
                "max_health_level",
            )?;
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
#[derive(Clone, Copy, Default)]
struct StatDeltas {
    damage: i64,
    attack_speed: i64,
    pierce: i64,
    crit: i64,
    armor: i64,
    max_health: i64,
}

#[cfg(feature = "server")]
impl StatDeltas {
    fn add(&mut self, kind: UpgradeStatKind, delta: i64) {
        match kind {
            UpgradeStatKind::Damage => self.damage += delta,
            UpgradeStatKind::AttackSpeed => self.attack_speed += delta,
            UpgradeStatKind::Pierce => self.pierce += delta,
            UpgradeStatKind::Crit => self.crit += delta,
            UpgradeStatKind::Armor => self.armor += delta,
            UpgradeStatKind::MaxHealth => self.max_health += delta,
        }
    }
}

#[cfg(feature = "server")]
impl Transaction {
    fn change(
        username: impl Into<String>,
        weapon_id: WeaponId,
        fragment_delta: i64,
        stats: StatDeltas,
        set_equipped: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            changes: vec![PlayerChange {
                username: username.into(),
                weapon_id,
                fragment_delta,
                damage_level_delta: stats.damage,
                attack_speed_level_delta: stats.attack_speed,
                pierce_level_delta: stats.pierce,
                crit_level_delta: stats.crit,
                armor_level_delta: stats.armor,
                max_health_level_delta: stats.max_health,
                set_equipped,
            }],
        }
    }

    pub fn add_weapon_fragments(
        username: impl Into<String>,
        weapon_id: WeaponId,
        amount: u32,
    ) -> Self {
        Self::change(
            username,
            weapon_id,
            amount as i64,
            StatDeltas::default(),
            false,
        )
    }

    pub fn upgrade_weapon_stat(
        username: impl Into<String>,
        weapon_id: WeaponId,
        kind: UpgradeStatKind,
        cost: u32,
    ) -> Self {
        let mut stats = StatDeltas::default();
        stats.add(kind, 1);
        Self::change(username, weapon_id, -(cost as i64), stats, false)
    }

    pub fn death_penalty(
        username: impl Into<String>,
        weapon_id: WeaponId,
        progress: &WeaponProgress,
        progression: &ProgressionSettings,
        rng: &mut impl rand::Rng,
    ) -> Option<Self> {
        if progress.fragments == 0 {
            return None;
        }

        let mut levels = progress.clone();
        let mut budget = progress.fragments;
        let mut stats = StatDeltas::default();
        loop {
            let candidates: Vec<(UpgradeStatKind, u32)> = UpgradeStatKind::ALL
                .into_iter()
                .filter_map(|kind| {
                    let level = levels.level_for(kind);
                    if level == 0 {
                        return None;
                    }
                    let cost = progression.upgrade_cost(level - 1).ok()?;
                    (cost <= budget).then_some((kind, cost))
                })
                .collect();
            if candidates.is_empty() {
                break;
            }
            let (kind, cost) = candidates[rng.random_range(0..candidates.len())];
            levels.set_level_for(kind, levels.level_for(kind) - 1);
            budget -= cost;
            stats.add(kind, -1);
        }

        Some(Self::change(
            username,
            weapon_id,
            -(progress.fragments as i64),
            stats,
            false,
        ))
    }

    pub fn equip_weapon(username: impl Into<String>, weapon_id: WeaponId) -> Self {
        Self::change(username, weapon_id, 0, StatDeltas::default(), true)
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
