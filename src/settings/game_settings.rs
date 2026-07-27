#[cfg(not(target_family = "wasm"))]
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::weapon::protocol::WeaponId;

#[derive(Resource, Debug, Clone, Deserialize)]
pub(crate) struct GameSettings {
    pub network: NetworkSettings,
    pub enemy: EnemySettings,
    pub spawner: SpawnerSettings,
    pub fragment: FragmentSettings,
    pub post_processing: PostProcessingSettings,
    pub camera: CameraSettings,
    pub hud: HudSettings,
    pub player_visual: PlayerVisualSettings,
    pub combat_feedback: CombatFeedbackSettings,
    pub arena_floor: ArenaFloorSettings,
    pub player: PlayerSettings,
    pub world: WorldSettings,
    pub gate: GateSettings,
    pub gate_visual: GateVisualSettings,
    pub server_camera: ServerCameraSettings,
    pub projectile: ProjectileSettings,
    pub weapons: WeaponsSettings,
    pub weapon_stations: WeaponStationsSettings,
    pub progression: ProgressionSettings,
    pub menu: MenuSettings,
    pub mobile_controls: MobileControlsSettings,
    pub bot: BotSettings,
    pub global_aristeia: GlobalAristeiaSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostProcessingSettings {
    #[serde(deserialize_with = "deserialize_color")]
    pub clear_color: Color,
    pub bloom_intensity: f32,
    pub bloom_low_frequency_boost: f32,
    #[serde(deserialize_with = "deserialize_color")]
    pub projectile_tint: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub fragment_tint: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub gate_tint: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub username_text: Color,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaFloorSettings {
    pub bands: Vec<ArenaFloorBandSettings>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaFloorBandSettings {
    #[serde(deserialize_with = "deserialize_color")]
    pub inner_tint: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub outer_tint: Color,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkSettings {
    pub interest_radius: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnemySettings {
    pub size: f32,
    pub collision_radius: f32,
    pub despawn_distance_from_players: f32,
    pub wander_radius: f32,
    pub detection_radius: f32,
    pub leash_radius: f32,
    pub standoff_distance: f32,
    pub strafe_radius: f32,
    pub engage_retarget_seconds: f32,
    pub ranged_chance: f32,
    pub attacks_per_second: f32,
    pub projectile_speed: f32,
    pub projectile_radius: f32,
    pub projectile_range: f32,
    pub projectile: WeaponSpriteSheetSettings,
    pub impact: WeaponSpriteSheetSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpawnerTierSettings {
    pub max_distance_from_center: f32,
    pub spawn_rate_multiplier: f32,
    pub max_owned_multiplier: f32,
    pub health: u32,
    pub contact_damage: u32,
    pub ranged_damage: u32,
    pub move_speed: f32,
    pub fragment_multiplier: f32,
    pub ranged_chance: f32,
    pub personal_aristeia_reward: u32,
    pub global_aristeia_reward: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpawnerSettings {
    pub count: usize,
    pub activation_radius: f32,
    pub max_owned: usize,
    pub spawn_radius: f32,
    pub spawn_rate_per_second: f32,
    pub placement_candidate_count: usize,
    pub spawn_min_distance_from_player: f32,
    pub spawn_candidate_count: usize,
    pub tiers: Vec<SpawnerTierSettings>,
}

impl SpawnerSettings {
    pub fn tier_index_for_distance(&self, distance_from_center: f32) -> u8 {
        self.tiers
            .iter()
            .position(|tier| distance_from_center <= tier.max_distance_from_center)
            .unwrap_or(self.tiers.len().saturating_sub(1)) as u8
    }

    pub fn tier(&self, index: u8) -> &SpawnerTierSettings {
        self.tiers
            .get(index as usize)
            .unwrap_or_else(|| &self.tiers[self.tiers.len() - 1])
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FragmentSettings {
    pub pool_min_radius: f32,
    pub pool_max_radius: f32,
    pub radius: f32,
    pub lifetime_ticks: u16,
    pub player_collection_radius: f32,
    pub pull_speed: f32,
    pub pull_acceleration: f32,
    pub swirl: f32,
    pub collecting_impact_ticks: u16,
    pub base_drop_count: u16,
    pub bonus_drops_per_aristeia: u16,
    pub maximum_drop_count: u16,
    pub drop_entity_count: u16,
    pub on_ground: WeaponSpriteSheetSettings,
    pub starting_pull: WeaponSpriteSheetSettings,
    pub moving: WeaponSpriteSheetSettings,
    pub collecting_impact: WeaponSpriteSheetSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CameraSettings {
    pub decay_rate: f32,
    pub scale: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HudSettings {
    pub width: f32,
    pub bar_height: f32,
    pub inset: f32,
    pub row_gap: f32,
    pub padding: f32,
    pub boss_indicator_size: f32,
    pub boss_indicator_margin: f32,
    pub boss_indicator_path: String,
    #[serde(deserialize_with = "deserialize_color")]
    pub boss_indicator_color: Color,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerVisualSettings {
    #[serde(deserialize_with = "deserialize_vec3")]
    pub username_label_offset: Vec3,
    pub aim_smooth_rate: f32,
    pub health_bar_width: f32,
    pub health_bar_height: f32,
    pub health_bar_offset_y: f32,
    pub held_weapon_offset: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CombatFeedbackSettings {
    pub flash_duration_seconds: f32,
    #[serde(deserialize_with = "deserialize_color")]
    pub flash_color: Color,
    pub popup_duration_seconds: f32,
    pub popup_rise_speed: f32,
    pub popup_offset_y: f32,
    pub popup_font_size: f32,
    #[serde(deserialize_with = "deserialize_color")]
    pub outgoing_color: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub incoming_color: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub crit_color: Color,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerSettings {
    pub maximum_health: u32,
    pub enemy_contact_damage_interval_seconds: f32,
    pub death_screen_duration_seconds: f32,
    pub half_size: f32,
    pub collision_radius: f32,
    pub move_speed: f32,
    pub arena_health_regen_fraction_per_second: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldSettings {
    pub tile_pixel_size: f32,
    pub arena_width_in_tiles: u32,
    pub arena_height_in_tiles: u32,
    pub safezone_width_in_tiles: u32,
    pub safezone_height_in_tiles: u32,
    pub safezone_gap_in_tiles: u32,
    pub stream_half_width: i32,
    pub stream_half_height: i32,
    pub arena_floor_tiles: Vec<usize>,
    pub arena_tileset: TileAtlasSettings,
    pub safezone: SafezoneSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileAtlasSettings {
    pub path: String,
    #[serde(deserialize_with = "deserialize_uvec2")]
    pub cell: UVec2,
    pub columns: u32,
    pub rows: u32,
}

impl TileAtlasSettings {
    fn is_valid(&self) -> bool {
        !self.path.is_empty()
            && self.cell.x > 0
            && self.cell.y > 0
            && self.columns > 0
            && self.rows > 0
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SafezoneSettings {
    #[serde(deserialize_with = "deserialize_color")]
    pub background_color: Color,
    pub wall_ring_thickness: i32,
    #[serde(deserialize_with = "deserialize_uvec2")]
    pub wall_tile: UVec2,
    pub tileset: TileAtlasSettings,
    pub floors: Vec<SafezoneFloorRect>,
    pub props: Vec<SafezoneProp>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SafezoneFloorRect {
    #[serde(deserialize_with = "deserialize_uvec2")]
    pub center_tile: UVec2,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SafezoneProp {
    #[serde(deserialize_with = "deserialize_uvec2")]
    pub tile: UVec2,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GateSettings {
    /// Radius for walking into an open gate to teleport.
    pub teleport_radius: f32,
    /// Radius for charging a closed arena gate / extended enemy aggro.
    pub nearby_radius: f32,
    pub arena_gate_count: usize,
    pub arena_edge_inset: f32,
    pub placement_attempts: usize,
    pub teleport_cooldown_ticks: u16,
    pub open_fill_seconds: f32,
    pub closed_decay_seconds: f32,
    pub open_drain_seconds: f32,
    /// Multiplier applied to enemy detection/leash when a player is within nearby_radius.
    pub nearby_detection_multiplier: f32,

    #[serde(deserialize_with = "deserialize_vec2")]
    pub safezone_gate_offset: Vec2,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GateVisualSettings {
    pub bar_width: f32,
    pub bar_height: f32,
    pub bar_offset_y: f32,
}

impl WorldSettings {
    pub fn arena_bounds(&self) -> Rect {
        Rect::from_center_size(
            Vec2::ZERO,
            Vec2::new(
                self.arena_width_in_tiles as f32 * self.tile_pixel_size,
                self.arena_height_in_tiles as f32 * self.tile_pixel_size,
            ),
        )
    }

    pub fn safezone_bounds(&self) -> Rect {
        let arena_width = self.arena_width_in_tiles as f32 * self.tile_pixel_size;
        let safezone_width = self.safezone_width_in_tiles as f32 * self.tile_pixel_size;
        let gap = self.safezone_gap_in_tiles as f32 * self.tile_pixel_size;
        Rect::from_center_size(
            Vec2::new(arena_width * 0.5 + safezone_width * 0.5 + gap, 0.0),
            Vec2::new(
                safezone_width,
                self.safezone_height_in_tiles as f32 * self.tile_pixel_size,
            ),
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerCameraSettings {
    pub zoom_speed: f32,
    pub min_scale: f32,
    pub max_scale: f32,
    pub pan_speed_in_screen_heights: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectileSettings {
    pub impact_lifetime_ticks: u16,
    pub spawn_gap: f32,
    pub buffer_capacity: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponsSettings(pub Vec<WeaponSettings>);

impl WeaponsSettings {
    pub fn default_id(&self) -> WeaponId {
        self.0[0].id
    }

    pub fn get(&self, id: WeaponId) -> Option<&WeaponSettings> {
        self.0.iter().find(|weapon| weapon.id == id)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponSettings {
    pub id: WeaponId,
    pub name: String,
    pub damage: u32,
    pub range: f32,
    pub attacks_per_second: f32,
    pub projectile_speed: f32,
    pub projectile_radius: f32,
    pub icon: String,
    pub projectile: WeaponSpriteSheetSettings,
    pub impact: WeaponSpriteSheetSettings,
}

impl WeaponSettings {
    fn is_valid(&self) -> bool {
        self.range > 0.0
            && self.attacks_per_second > 0.0
            && self.projectile_speed > 0.0
            && self.projectile_radius >= 0.0
            && !self.name.is_empty()
            && !self.icon.is_empty()
            && self.projectile.is_valid()
            && self.impact.is_valid()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponSpriteSheetSettings {
    pub path: String,
    #[serde(deserialize_with = "deserialize_uvec2")]
    pub cell: UVec2,
    pub frames: u32,
    pub frame_seconds: f32,
}

impl WeaponSpriteSheetSettings {
    fn is_valid(&self) -> bool {
        !self.path.is_empty()
            && self.cell.x > 0
            && self.cell.y > 0
            && self.frames > 0
            && self.frame_seconds > 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeStatKind {
    Damage,
    AttackSpeed,
    Pierce,
    Crit,
    Armor,
    MaxHealth,
}

impl UpgradeStatKind {
    pub const ALL: [Self; 6] = [
        Self::Damage,
        Self::AttackSpeed,
        Self::Pierce,
        Self::Crit,
        Self::Armor,
        Self::MaxHealth,
    ];
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponStationsSettings {
    pub trigger_radius: f32,
    pub stations: Vec<WeaponStationSettings>,
    pub upgrade_stations: Vec<UpgradeStationSettings>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponStationSettings {
    pub weapon_id: WeaponId,
    #[serde(deserialize_with = "deserialize_vec2")]
    pub offset: Vec2,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpgradeStationSettings {
    pub kind: UpgradeStatKind,
    #[serde(deserialize_with = "deserialize_vec2")]
    pub offset: Vec2,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProgressionSettings {
    pub damage_bonus_per_level: f32,
    pub attack_speed_bonus_per_level: f32,
    pub pierce_per_level: u16,
    pub crit_chance_per_level: f32,
    pub crit_damage_multiplier: f32,
    pub armor_per_level: u32,
    pub max_health_per_level: u32,
    pub upgrade_cost_base: u32,
    pub upgrade_cost_level_scale: u32,
    pub upgrade_cost_level_power: u32,
}

impl ProgressionSettings {
    pub fn damage_at(&self, base_damage: u32, level: u32) -> u32 {
        (base_damage as f32 * (1.0 + self.damage_bonus_per_level * level as f32)).round() as u32
    }

    pub fn attacks_per_second_at(&self, base_aps: f32, level: u32) -> f32 {
        base_aps * (1.0 + self.attack_speed_bonus_per_level * level as f32)
    }

    pub fn pierce_at(&self, level: u32) -> u16 {
        self.pierce_per_level.saturating_mul(level as u16)
    }

    pub fn crit_chance_at(&self, level: u32) -> f32 {
        (self.crit_chance_per_level * level as f32).clamp(0.0, 1.0)
    }

    pub fn armor_at(&self, level: u32) -> u32 {
        self.armor_per_level.saturating_mul(level)
    }

    pub fn max_health_at(&self, base_health: u32, level: u32) -> u32 {
        base_health.saturating_add(self.max_health_per_level.saturating_mul(level))
    }

    pub fn upgrade_cost(&self, level: u32) -> Result<u32, String> {
        let raised = level
            .checked_pow(self.upgrade_cost_level_power)
            .ok_or_else(|| format!("upgrade cost overflow for level {level}"))?;
        let scaled = raised
            .checked_mul(self.upgrade_cost_level_scale)
            .ok_or_else(|| format!("upgrade cost overflow for level {level}"))?;
        scaled
            .checked_add(self.upgrade_cost_base)
            .ok_or_else(|| format!("upgrade cost overflow for level {level}"))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BossPatternSettings {
    pub duration_seconds: f32,
    pub projectile_count: u16,
    pub volley_interval_seconds: f32,
    #[serde(default)]
    pub spread_degrees: f32,
    #[serde(default)]
    pub rotation_degrees: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BossPatternsSettings {
    pub fan: BossPatternSettings,
    pub radial: BossPatternSettings,
    pub spiral: BossPatternSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalAristeiaSettings {
    pub threshold: u32,
    pub drain_interval_ticks: u16,
    pub drain_amount: u32,
    pub personal_level_cost_base: f32,
    pub personal_level_cost_scale: f32,
    pub personal_level_cost_exponent: f32,
    pub personal_duration_base_seconds: f32,
    pub personal_duration_decay_per_level: f32,
    pub personal_duration_min_seconds: f32,
    pub grand_champion_health: u32,
    pub grand_champion_move_speed: f32,
    pub grand_champion_spawn_distance: f32,
    pub grand_champion_detection_radius: f32,
    pub grand_champion_leash_radius: f32,
    pub grand_champion_contact_damage: u32,
    pub grand_champion_ranged_damage: u32,
    pub grand_champion_fragment_multiplier: f32,
    pub grand_champion_personal_aristeia_reward: u32,
    pub grand_champion_projectile_speed: f32,
    pub grand_champion_projectile_radius: f32,
    pub grand_champion_projectile_range: f32,
    pub grand_champion_projectile_buffer_capacity: usize,
    pub patterns: BossPatternsSettings,
}

impl GlobalAristeiaSettings {
    pub fn personal_level_cost(&self, level: u32) -> u32 {
        (self.personal_level_cost_base
            + self.personal_level_cost_scale
                * (level as f32).powf(self.personal_level_cost_exponent))
        .ceil()
        .max(1.0) as u32
    }

    pub fn personal_duration_seconds(&self, level: u32) -> f32 {
        let scaled = self.personal_duration_base_seconds
            / (1.0 + self.personal_duration_decay_per_level * level as f32);
        scaled.max(self.personal_duration_min_seconds)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MenuSettings {
    #[serde(deserialize_with = "deserialize_color")]
    pub normal_button: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub hovered_button: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub pressed_button: Color,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MobileControlsSettings {
    pub joystick_size: f32,
    pub knob_size: f32,
    pub screen_inset: f32,
    pub movement_dead_zone: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BotSettings {
    pub arena_portal_seek_chance: f32,
    pub safezone_portal_seek_chance: f32,
    pub move_re_roll_min_seconds: f32,
    pub move_re_roll_max_seconds: f32,
}

impl GameSettings {
    const SETTINGS_FILES: &[&str] = &[
        "world.json",
        "player.json",
        "enemies.json",
        "graphics.json",
        "bots.json",
    ];

    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameSettingsLoadError> {
        let directory = path.as_ref();
        let mut merged = serde_json::Map::new();

        for file_name in Self::SETTINGS_FILES {
            let file_path = directory.join(file_name);
            let json = Self::read_settings_file(&file_path, file_name)?;
            let value = serde_json::from_str::<serde_json::Value>(&json).map_err(|source| {
                GameSettingsLoadError::Parse {
                    path: file_path.clone(),
                    source,
                }
            })?;
            let serde_json::Value::Object(map) = value else {
                return Err(GameSettingsLoadError::Invalid(format!(
                    "settings file {} must contain a JSON object",
                    file_path.display()
                )));
            };
            for (key, nested) in map {
                if merged.insert(key.clone(), nested).is_some() {
                    return Err(GameSettingsLoadError::Invalid(format!(
                        "duplicate settings key '{key}' while loading {}",
                        file_path.display()
                    )));
                }
            }
        }

        let settings = serde_json::from_value::<GameSettings>(serde_json::Value::Object(merged))
            .map_err(|source| GameSettingsLoadError::Parse {
                path: directory.to_path_buf(),
                source,
            })?;

        settings.validate()?;
        Ok(settings)
    }

    fn read_settings_file(
        file_path: &Path,
        file_name: &str,
    ) -> Result<String, GameSettingsLoadError> {
        // wasm32-unknown-unknown has no filesystem, so settings are embedded.
        #[cfg(target_family = "wasm")]
        {
            let _ = file_path;
            let json = match file_name {
                "world.json" => include_str!("../../assets/settings/world.json"),
                "player.json" => include_str!("../../assets/settings/player.json"),
                "enemies.json" => include_str!("../../assets/settings/enemies.json"),
                "graphics.json" => include_str!("../../assets/settings/graphics.json"),
                "bots.json" => include_str!("../../assets/settings/bots.json"),
                _ => {
                    return Err(GameSettingsLoadError::Invalid(format!(
                        "unknown embedded settings file {file_name}"
                    )));
                }
            };
            Ok(json.to_owned())
        }

        #[cfg(not(target_family = "wasm"))]
        {
            let _ = file_name;
            fs::read_to_string(file_path).map_err(|source| GameSettingsLoadError::Read {
                path: file_path.to_path_buf(),
                source,
            })
        }
    }

    fn validate(&self) -> Result<(), GameSettingsLoadError> {
        if self.network.interest_radius <= 0.0 {
            return Err(GameSettingsLoadError::Invalid(
                "network interest_radius must be greater than zero".into(),
            ));
        }
        if self.enemy.size <= 0.0 || self.enemy.collision_radius <= 0.0 {
            return Err(GameSettingsLoadError::Invalid(
                "enemy size and collision radius must be greater than zero".into(),
            ));
        }
        if self.enemy.detection_radius <= 0.0
            || self.enemy.despawn_distance_from_players <= 0.0
            || self.enemy.leash_radius < self.enemy.detection_radius
            || self.enemy.leash_radius < self.enemy.wander_radius
            || self.enemy.standoff_distance < 0.0
            || self.enemy.strafe_radius < 0.0
            || self.enemy.engage_retarget_seconds <= 0.0
            || !(0.0..=1.0).contains(&self.enemy.ranged_chance)
            || self.enemy.attacks_per_second <= 0.0
            || self.enemy.projectile_speed <= 0.0
            || self.enemy.projectile_radius < 0.0
            || self.enemy.projectile_range <= 0.0
            || !self.enemy.projectile.is_valid()
            || !self.enemy.impact.is_valid()
        {
            return Err(GameSettingsLoadError::Invalid(
                "enemy movement or attack settings are invalid".into(),
            ));
        }
        if self.spawner.count == 0
            || self.spawner.activation_radius <= 0.0
            || self.spawner.max_owned == 0
            || self.spawner.spawn_radius <= 0.0
            || self.spawner.spawn_rate_per_second <= 0.0
            || self.spawner.placement_candidate_count == 0
            || self.spawner.spawn_candidate_count == 0
            || self.spawner.spawn_min_distance_from_player < 0.0
            || self.enemy.despawn_distance_from_players
                <= self.spawner.spawn_min_distance_from_player
            || self.spawner.tiers.is_empty()
        {
            return Err(GameSettingsLoadError::Invalid(
                "spawner settings are invalid".into(),
            ));
        }
        let mut previous_distance = 0.0;
        for (index, tier) in self.spawner.tiers.iter().enumerate() {
            if tier.max_distance_from_center < previous_distance
                || tier.spawn_rate_multiplier <= 0.0
                || tier.max_owned_multiplier <= 0.0
                || tier.health == 0
                || tier.move_speed <= 0.0
                || tier.fragment_multiplier < 0.0
                || !(0.0..=1.0).contains(&tier.ranged_chance)
                || tier.personal_aristeia_reward == 0
                || tier.global_aristeia_reward == 0
            {
                return Err(GameSettingsLoadError::Invalid(format!(
                    "spawner tier {index} settings are invalid"
                )));
            }
            previous_distance = tier.max_distance_from_center;
        }
        if self.fragment.pool_min_radius > self.fragment.pool_max_radius {
            return Err(GameSettingsLoadError::Invalid(
                "fragment pool_min_radius cannot exceed pool_max_radius".into(),
            ));
        }
        if self.fragment.pull_speed < 0.0 || self.fragment.pull_acceleration < 0.0 {
            return Err(GameSettingsLoadError::Invalid(
                "fragment pull_speed and pull_acceleration cannot be negative".into(),
            ));
        }
        if self.fragment.base_drop_count > self.fragment.maximum_drop_count
            || self.fragment.drop_entity_count == 0
            || self.fragment.collecting_impact_ticks == 0
            || !self.fragment.on_ground.is_valid()
            || !self.fragment.starting_pull.is_valid()
            || !self.fragment.moving.is_valid()
            || !self.fragment.collecting_impact.is_valid()
            || self.global_aristeia.threshold == 0
            || self.global_aristeia.drain_interval_ticks == 0
            || self.global_aristeia.personal_level_cost_base <= 0.0
            || self.global_aristeia.personal_level_cost_scale < 0.0
            || self.global_aristeia.personal_level_cost_exponent <= 0.0
            || self.global_aristeia.personal_duration_base_seconds <= 0.0
            || self.global_aristeia.personal_duration_decay_per_level < 0.0
            || self.global_aristeia.personal_duration_min_seconds <= 0.0
            || self.global_aristeia.personal_duration_min_seconds
                > self.global_aristeia.personal_duration_base_seconds
        {
            return Err(GameSettingsLoadError::Invalid(
                "fragment drop, visual, or global Aristeia settings are invalid".into(),
            ));
        }
        if self.progression.damage_bonus_per_level < 0.0
            || self.progression.attack_speed_bonus_per_level < 0.0
            || self.progression.crit_chance_per_level < 0.0
            || self.progression.crit_damage_multiplier < 1.0
        {
            return Err(GameSettingsLoadError::Invalid(
                "progression damage/attack-speed/crit settings are invalid".into(),
            ));
        }
        if self.progression.upgrade_cost_base == 0 || self.progression.upgrade_cost_level_scale == 0
        {
            return Err(GameSettingsLoadError::Invalid(
                "progression upgrade cost coefficients must be greater than zero".into(),
            ));
        }
        if self.global_aristeia.grand_champion_health == 0
            || self.global_aristeia.grand_champion_move_speed <= 0.0
            || self.global_aristeia.grand_champion_spawn_distance <= 0.0
            || self.global_aristeia.grand_champion_detection_radius <= 0.0
            || self.global_aristeia.grand_champion_leash_radius
                < self.global_aristeia.grand_champion_detection_radius
            || self.global_aristeia.grand_champion_fragment_multiplier < 0.0
            || self.global_aristeia.grand_champion_personal_aristeia_reward == 0
            || self.global_aristeia.grand_champion_projectile_speed <= 0.0
            || self.global_aristeia.grand_champion_projectile_radius < 0.0
            || self.global_aristeia.grand_champion_projectile_range <= 0.0
            || self
                .global_aristeia
                .grand_champion_projectile_buffer_capacity
                == 0
            || !boss_pattern_valid(&self.global_aristeia.patterns.fan)
            || !boss_pattern_valid(&self.global_aristeia.patterns.radial)
            || !boss_pattern_valid(&self.global_aristeia.patterns.spiral)
        {
            return Err(GameSettingsLoadError::Invalid(
                "global Aristeia champion settings are invalid".into(),
            ));
        }
        if self.world.tile_pixel_size <= 0.0
            || self.world.arena_width_in_tiles == 0
            || self.world.arena_height_in_tiles == 0
            || self.world.safezone_width_in_tiles == 0
            || self.world.safezone_height_in_tiles == 0
        {
            return Err(GameSettingsLoadError::Invalid(
                "world tile size and dimensions must be greater than zero".into(),
            ));
        }
        if self.world.arena_floor_tiles.is_empty() {
            return Err(GameSettingsLoadError::Invalid(
                "world arena_floor_tiles cannot be empty".into(),
            ));
        }
        if !self.world.arena_tileset.is_valid() || !self.world.safezone.tileset.is_valid() {
            return Err(GameSettingsLoadError::Invalid(
                "world arena/safezone tileset settings are invalid".into(),
            ));
        }
        if self.world.safezone.wall_ring_thickness < 0 {
            return Err(GameSettingsLoadError::Invalid(
                "safezone wall_ring_thickness cannot be negative".into(),
            ));
        }
        for (index, floor) in self.world.safezone.floors.iter().enumerate() {
            if floor.width <= 0 || floor.height <= 0 {
                return Err(GameSettingsLoadError::Invalid(format!(
                    "safezone floor {index} width and height must be greater than zero"
                )));
            }
        }
        if self.post_processing.bloom_intensity < 0.0
            || self.post_processing.bloom_low_frequency_boost < 0.0
        {
            return Err(GameSettingsLoadError::Invalid(
                "post_processing bloom_intensity and bloom_low_frequency_boost cannot be negative"
                    .into(),
            ));
        }
        if self.camera.scale <= 0.0 || self.camera.decay_rate < 0.0 {
            return Err(GameSettingsLoadError::Invalid(
                "camera scale must be greater than zero and decay_rate cannot be negative".into(),
            ));
        }
        if self.arena_floor.bands.len() != self.spawner.tiers.len() {
            return Err(GameSettingsLoadError::Invalid(format!(
                "arena_floor bands ({}) must match spawner tiers ({})",
                self.arena_floor.bands.len(),
                self.spawner.tiers.len()
            )));
        }
        if self.hud.width <= 0.0
            || self.hud.bar_height <= 0.0
            || self.hud.inset < 0.0
            || self.hud.row_gap < 0.0
            || self.hud.padding < 0.0
            || self.hud.boss_indicator_size <= 0.0
            || self.hud.boss_indicator_margin < 0.0
            || self.hud.boss_indicator_path.is_empty()
        {
            return Err(GameSettingsLoadError::Invalid(
                "hud layout settings are invalid".into(),
            ));
        }
        if self.player_visual.aim_smooth_rate < 0.0
            || self.player_visual.health_bar_width <= 0.0
            || self.player_visual.health_bar_height <= 0.0
            || self.player_visual.held_weapon_offset < 0.0
        {
            return Err(GameSettingsLoadError::Invalid(
                "player_visual settings are invalid".into(),
            ));
        }
        if self.combat_feedback.flash_duration_seconds <= 0.0
            || self.combat_feedback.popup_duration_seconds <= 0.0
            || self.combat_feedback.popup_rise_speed < 0.0
            || self.combat_feedback.popup_font_size <= 0.0
        {
            return Err(GameSettingsLoadError::Invalid(
                "combat_feedback settings are invalid".into(),
            ));
        }
        if self.player.maximum_health == 0
            || self.player.death_screen_duration_seconds <= 0.0
            || self.player.enemy_contact_damage_interval_seconds <= 0.0
            || self.player.arena_health_regen_fraction_per_second < 0.0
            || self.player.half_size <= 0.0
            || self.player.collision_radius <= 0.0
            || self.player.move_speed <= 0.0
        {
            return Err(GameSettingsLoadError::Invalid(
                "player gameplay settings are invalid".into(),
            ));
        }
        if self.gate.teleport_radius <= 0.0
            || self.gate.nearby_radius <= 0.0
            || self.gate.arena_gate_count == 0
            || self.gate.arena_edge_inset < 0.0
            || self.gate.placement_attempts == 0
            || self.gate.teleport_cooldown_ticks == 0
            || self.gate.open_fill_seconds <= 0.0
            || self.gate.closed_decay_seconds <= 0.0
            || self.gate.open_drain_seconds <= 0.0
            || self.gate.nearby_detection_multiplier < 1.0
        {
            return Err(GameSettingsLoadError::Invalid(
                "gate settings are invalid".into(),
            ));
        }
        if self.gate_visual.bar_width <= 0.0 || self.gate_visual.bar_height <= 0.0 {
            return Err(GameSettingsLoadError::Invalid(
                "gate_visual bar dimensions must be greater than zero".into(),
            ));
        }
        if self.server_camera.min_scale <= 0.0
            || self.server_camera.min_scale > self.server_camera.max_scale
        {
            return Err(GameSettingsLoadError::Invalid(
                "server camera scale range is invalid".into(),
            ));
        }
        if self.projectile.buffer_capacity == 0
            || self.projectile.impact_lifetime_ticks == 0
            || self.projectile.spawn_gap < 0.0
        {
            return Err(GameSettingsLoadError::Invalid(
                "projectile buffer capacity, impact lifetime, and spawn gap are invalid".into(),
            ));
        }
        if self.weapons.0.is_empty() {
            return Err(GameSettingsLoadError::Invalid(
                "weapons list cannot be empty".into(),
            ));
        }
        let mut seen_weapon_ids = std::collections::HashSet::new();
        for weapon in &self.weapons.0 {
            if !seen_weapon_ids.insert(weapon.id) {
                return Err(GameSettingsLoadError::Invalid(format!(
                    "duplicate weapon id {}",
                    weapon.id
                )));
            }
            if !weapon.is_valid() {
                return Err(GameSettingsLoadError::Invalid(format!(
                    "weapon settings are invalid for id {}",
                    weapon.id
                )));
            }
        }
        if self.weapon_stations.trigger_radius <= 0.0 {
            return Err(GameSettingsLoadError::Invalid(
                "weapon station trigger radius must be greater than zero".into(),
            ));
        }
        if self.mobile_controls.joystick_size <= 0.0
            || self.mobile_controls.knob_size <= 0.0
            || self.mobile_controls.knob_size > self.mobile_controls.joystick_size
            || self.mobile_controls.screen_inset < 0.0
            || !(0.0..1.0).contains(&self.mobile_controls.movement_dead_zone)
        {
            return Err(GameSettingsLoadError::Invalid(
                "mobile controls joystick_size, knob_size, screen_inset, and movement_dead_zone are invalid".into(),
            ));
        }
        if !(0.0..=1.0).contains(&self.bot.arena_portal_seek_chance)
            || !(0.0..=1.0).contains(&self.bot.safezone_portal_seek_chance)
            || self.bot.move_re_roll_min_seconds <= 0.0
            || self.bot.move_re_roll_max_seconds <= self.bot.move_re_roll_min_seconds
        {
            return Err(GameSettingsLoadError::Invalid(
                "bot portal_seek_chance and move_re_roll_*_seconds are invalid".into(),
            ));
        }
        for station in &self.weapon_stations.stations {
            if !seen_weapon_ids.contains(&station.weapon_id) {
                return Err(GameSettingsLoadError::Invalid(format!(
                    "weapon station references unknown weapon id {}",
                    station.weapon_id
                )));
            }
        }
        if self.weapon_stations.upgrade_stations.is_empty() {
            return Err(GameSettingsLoadError::Invalid(
                "weapon stations must include at least one upgrade station".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum GameSettingsLoadError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    Invalid(String),
}

impl std::fmt::Display for GameSettingsLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read settings file {}: {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                write!(
                    formatter,
                    "failed to parse settings file {}: {source}",
                    path.display()
                )
            }
            Self::Invalid(message) => {
                write!(formatter, "invalid game settings: {message}")
            }
        }
    }
}

impl std::error::Error for GameSettingsLoadError {}

fn deserialize_vec2<'de, D>(deserializer: D) -> Result<Vec2, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let [x, y] = <[f32; 2]>::deserialize(deserializer)?;
    Ok(Vec2::new(x, y))
}

fn deserialize_vec3<'de, D>(deserializer: D) -> Result<Vec3, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let [x, y, z] = <[f32; 3]>::deserialize(deserializer)?;
    Ok(Vec3::new(x, y, z))
}

fn deserialize_uvec2<'de, D>(deserializer: D) -> Result<UVec2, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let [x, y] = <[u32; 2]>::deserialize(deserializer)?;
    Ok(UVec2::new(x, y))
}

fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let [red, green, blue] = <[f32; 3]>::deserialize(deserializer)?;
    Ok(Color::srgb(red, green, blue))
}

fn boss_pattern_valid(pattern: &BossPatternSettings) -> bool {
    pattern.duration_seconds > 0.0
        && pattern.projectile_count > 0
        && pattern.volley_interval_seconds > 0.0
}
