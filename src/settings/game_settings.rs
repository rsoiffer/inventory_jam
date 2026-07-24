#[cfg(not(target_family = "wasm"))]
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::Deserialize;

use crate::weapon::protocol::WeaponId;

#[derive(Resource, Debug, Clone, Deserialize)]
pub(crate) struct GameSettings {
    pub network: NetworkSettings,
    pub enemy: EnemySettings,
    pub spawner: SpawnerSettings,
    pub fragment: FragmentSettings,
    pub player: PlayerSettings,
    pub world: WorldSettings,
    pub portal: PortalSettings,
    pub server_camera: ServerCameraSettings,
    pub projectile: ProjectileSettings,
    pub weapons: WeaponsSettings,
    pub weapon_stations: WeaponStationsSettings,
    pub progression: ProgressionSettings,
    pub menu: MenuSettings,
    pub mobile_controls: MobileControlsSettings,
    pub global_aristeia: GlobalAristeiaSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkSettings {
    pub interest_radius: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnemySettings {
    pub size: f32,
    pub max_health: u32,
    pub collision_radius: f32,
    pub despawn_distance_from_players: f32,
    pub move_speed: f32,
    pub wander_radius: f32,
    pub detection_radius: f32,
    pub leash_radius: f32,
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
    pub base_drop_count: u16,
    pub bonus_drops_per_aristeia: u16,
    pub maximum_drop_count: u16,
    pub drop_entity_count: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerSettings {
    pub camera_decay_rate: f32,
    pub camera_scale: f32,

    #[serde(deserialize_with = "deserialize_vec3")]
    pub username_label_offset: Vec3,

    pub aim_visual_smooth_rate: f32,
    pub health_bar_width: f32,
    pub health_bar_height: f32,
    pub health_bar_offset_y: f32,
    #[allow(unused)]
    pub spawn_attempts: usize,
    pub maximum_health: u32,
    pub enemy_contact_damage: u32,
    pub enemy_contact_damage_interval_seconds: f32,
    pub death_screen_duration_seconds: f32,
    pub half_size: f32,
    pub collision_radius: f32,
    pub move_speed: f32,
    #[allow(unused)]
    pub held_weapon_offset: f32,
    pub aristeia_duration_ticks: u16,
    pub aristeia_bar_width: f32,
    pub aristeia_bar_height: f32,
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct PortalSettings {
    pub trigger_radius: f32,
    pub arena_portal_count: usize,
    pub arena_edge_inset: f32,
    pub placement_attempts: usize,
    pub teleport_cooldown_ticks: u16,

    #[serde(deserialize_with = "deserialize_vec2")]
    pub safezone_portal_offset: Vec2,
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

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponStationsSettings {
    pub trigger_radius: f32,
    #[serde(deserialize_with = "deserialize_vec2")]
    pub upgrade_station_offset: Vec2,
    pub stations: Vec<WeaponStationSettings>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponStationSettings {
    pub weapon_id: WeaponId,
    #[serde(deserialize_with = "deserialize_vec2")]
    pub offset: Vec2,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProgressionSettings {
    pub damage_bonus_per_level: f32,
    pub upgrade_cost_base: u32,
    pub upgrade_cost_level_scale: u32,
    pub upgrade_cost_level_power: u32,
}

impl ProgressionSettings {
    pub fn damage_at(&self, base_damage: u32, level: u32) -> u32 {
        (base_damage as f32 * (1.0 + self.damage_bonus_per_level * level as f32)).round() as u32
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
pub struct GlobalAristeiaSettings {
    pub threshold: u32,
    pub contribution_per_kill: u32,
    pub drain_interval_ticks: u16,
    pub drain_amount: u32,
    pub grand_champion_health: u32,
    pub grand_champion_move_speed: f32,
    pub grand_champion_detection_radius: f32,
    pub grand_champion_leash_radius: f32,
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

impl GameSettings {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameSettingsLoadError> {
        let path = path.as_ref();

        // wasm32-unknown-unknown has no filesystem, so the settings file is
        // embedded into the binary at compile time instead of read at runtime.
        #[cfg(target_family = "wasm")]
        let json = include_str!("../../assets/game_settings.json").to_owned();

        #[cfg(not(target_family = "wasm"))]
        let json = fs::read_to_string(path).map_err(|source| GameSettingsLoadError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        let settings = serde_json::from_str::<GameSettings>(&json).map_err(|source| {
            GameSettingsLoadError::Parse {
                path: path.to_path_buf(),
                source,
            }
        })?;

        settings.validate()?;
        Ok(settings)
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
        if self.enemy.move_speed <= 0.0
            || self.enemy.detection_radius <= 0.0
            || self.enemy.despawn_distance_from_players <= 0.0
            || self.enemy.leash_radius < self.enemy.detection_radius
            || self.enemy.leash_radius < self.enemy.wander_radius
        {
            return Err(GameSettingsLoadError::Invalid(
                "enemy movement settings are invalid".into(),
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
        {
            return Err(GameSettingsLoadError::Invalid(
                "spawner settings are invalid".into(),
            ));
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
            || self.global_aristeia.threshold == 0
            || self.global_aristeia.drain_interval_ticks == 0
        {
            return Err(GameSettingsLoadError::Invalid(
                "fragment drop or global Aristeia settings are invalid".into(),
            ));
        }
        if self.progression.damage_bonus_per_level < 0.0 {
            return Err(GameSettingsLoadError::Invalid(
                "progression damage_bonus_per_level cannot be negative".into(),
            ));
        }
        if self.progression.upgrade_cost_base == 0 || self.progression.upgrade_cost_level_scale == 0
        {
            return Err(GameSettingsLoadError::Invalid(
                "progression upgrade cost coefficients must be greater than zero".into(),
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
        if self.player.camera_scale <= 0.0
            || self.player.maximum_health == 0
            || self.player.death_screen_duration_seconds <= 0.0
        {
            return Err(GameSettingsLoadError::Invalid(
                "player camera_scale, maximum_health, and death_screen_duration_seconds must be greater than zero".into(),
            ));
        }
        if self.portal.trigger_radius <= 0.0
            || self.portal.arena_portal_count == 0
            || self.portal.arena_edge_inset < 0.0
            || self.portal.placement_attempts == 0
            || self.portal.teleport_cooldown_ticks == 0
        {
            return Err(GameSettingsLoadError::Invalid(
                "portal settings are invalid".into(),
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
        for station in &self.weapon_stations.stations {
            if !seen_weapon_ids.contains(&station.weapon_id) {
                return Err(GameSettingsLoadError::Invalid(format!(
                    "weapon station references unknown weapon id {}",
                    station.weapon_id
                )));
            }
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
